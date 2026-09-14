//! Process stages launched by scripts/creator-workflow/run.py.
//! Real actor handlers and disk I/O; no Tauri webview or IPC mocks.
use super::tests::dispatch;
use super::*;

const SOURCE: &str = include_str!("../../../../scripts/creator-workflow/main.lua");

#[test]
#[ignore = "run through scripts/creator-workflow/run.py"]
fn creator_workflow_stage() {
    let root = PathBuf::from(std::env::var_os("CAIVEN_WORKFLOW_DIR").expect("workflow directory"));
    let project = root.join("My first game");
    let stage = std::env::var("CAIVEN_WORKFLOW_STAGE").expect("workflow stage");
    let mut studio = StudioCore::new(None).expect("studio core");
    match stage.as_str() {
        "save" => {
            dispatch(&mut studio, |reply| CoreCommand::NewProject {
                path: project.clone(),
                template_id: "blank".into(),
                reply,
            })
            .expect("create project");
            dispatch(&mut studio, |reply| CoreCommand::CreateModule {
                name: "settings.lua".into(),
                reply,
            })
            .expect("create module");
            for (name, text) in [
                ("main.lua", SOURCE),
                ("settings.lua", "return { start_x = 16 }\n"),
            ] {
                dispatch(&mut studio, |reply| CoreCommand::WriteBuffer {
                    path: project.join(name).display().to_string(),
                    text: text.into(),
                    reply,
                })
                .expect("edit source buffer");
            }
            dispatch(&mut studio, |reply| CoreCommand::WriteSprite {
                sprite: 0,
                pixels: vec![7; 64],
                reply,
            })
            .expect("edit sprite");
            dispatch(&mut studio, |reply| CoreCommand::WritePalette {
                slot: 7,
                hex: "#123456".into(),
                reply,
            })
            .expect("edit palette");
            dispatch(&mut studio, |reply| CoreCommand::WriteMemory {
                address: caiven_core::memory::SFX_RAM_BASE,
                bytes: vec![48, 15, 8, 0],
                reply,
            })
            .expect("edit sound");
            dispatch(&mut studio, CoreCommand::Save).expect("save project");
            assert!(studio.sources.iter().all(|source| !source.dirty));
            dispatch(&mut studio, CoreCommand::CloseProject).expect("close project");
            assert!(studio.cart.is_none());
        }
        "reopen" => {
            dispatch(&mut studio, |reply| CoreCommand::OpenProject {
                path: project.join("caiven.toml"),
                reply,
            })
            .expect("reopen saved project in fresh process");
            assert_eq!(studio.sources.len(), 2);
            assert_eq!(studio.sources[0].text, SOURCE);
            assert_eq!(studio.sources[1].text, "return { start_x = 16 }\n");
            assert_eq!(studio.console.vm.peek_memory(SPRITE_SHEET_RAM_BASE), 7);
            assert_eq!(studio.console.vm.get_palette()[7].to_rgb(), [18, 52, 86]);
            for (offset, byte) in [48, 15, 8, 0].into_iter().enumerate() {
                assert_eq!(
                    studio
                        .console
                        .vm
                        .peek_memory(caiven_core::memory::SFX_RAM_BASE + offset),
                    byte
                );
            }
            dispatch(&mut studio, |reply| CoreCommand::Export {
                path: root.join("game.cav"),
                reply,
            })
            .expect("export cartridge");
            dispatch(&mut studio, |reply| CoreCommand::ExportWeb {
                path: root.join("game.html"),
                reply,
            })
            .expect("export offline HTML");
        }
        _ => panic!("unknown workflow stage: {stage}"),
    }
}
