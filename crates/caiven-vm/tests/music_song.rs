//! Song-order playback: `play_music_song` chains the music bank's order
//! table into a song, honoring its loop point, and treats a bank written
//! before songs existed (all-zero order table) as "no song".

use caiven_cart::{CartSection, SectionKind, encode_asset_bank};
use caiven_core::memory::{
    MUSIC_LOOP_POINT_OFFSET, MUSIC_ORDER_OFFSET, MUSIC_PATTERN_DATA_LEN, MUSIC_PATTERN_ROWS,
    MUSIC_RAM_BASE,
};
use caiven_vm::input::Input;
use caiven_vm::rendering::font::Font;
use caiven_vm::{Vm, VmConfig};

/// `tick_audio_players` calls that advance playback by one whole pattern.
/// One row costs `ticks_per_row` ticks; a pattern is `MUSIC_PATTERN_ROWS` of
/// them, and the wrap to the next song step lands on the last of those.
fn ticks_per_pattern(vm: &Vm) -> usize {
    MUSIC_PATTERN_ROWS * vm.music_player().ticks_per_row as usize
}

/// Boots a cart that starts the song on its first frame, and returns the VM
/// with playback already armed (row 0, no ticks consumed yet).
fn vm_playing_song(order: &[u8], loop_byte: u8) -> Vm {
    let mut vm = Vm::new(VmConfig::default());
    let input = Input::new();
    let font = Font::empty();

    vm.load_section_to_ram(MUSIC_RAM_BASE + MUSIC_ORDER_OFFSET, order);
    vm.load_section_to_ram(MUSIC_RAM_BASE + MUSIC_LOOP_POINT_OFFSET, &[loop_byte]);
    vm.load_lua_source(
        r#"
        function _update()
          if not started then
            started = true
            play_music_song(0)
          end
        end
        "#,
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));

    vm.run_frame(&input, &font);
    vm
}

#[test]
fn song_advances_through_the_order_table_then_stops_without_a_loop() {
    // Step 0 plays pattern 1, step 1 plays pattern 3, nothing after that.
    let mut vm = vm_playing_song(&[2, 4], 0);
    let per_pattern = ticks_per_pattern(&vm);

    assert!(vm.music_player().active, "song should start playing");
    assert!(vm.music_player().song_active, "song mode should be on");
    assert_eq!(vm.music_player().pattern_id, 1);
    assert_eq!(vm.music_player().song_step, 0);

    for _ in 0..per_pattern {
        vm.tick_audio_players();
    }
    assert_eq!(
        vm.music_player().pattern_id,
        3,
        "first pattern should hand off to the second order step"
    );
    assert_eq!(vm.music_player().song_step, 1);
    assert_eq!(vm.music_player().row, 0);
    assert!(vm.music_player().active);

    for _ in 0..per_pattern {
        vm.tick_audio_players();
    }
    assert!(
        !vm.music_player().active,
        "a song with no loop point stops when the order table runs out"
    );
    assert!(!vm.music_player().song_active);
}

#[test]
fn song_with_a_loop_point_keeps_playing_indefinitely() {
    // Two steps (patterns 0 and 1), looping back to step 0.
    let mut vm = vm_playing_song(&[1, 2], 1);
    let per_pattern = ticks_per_pattern(&vm);

    assert_eq!(vm.music_player().pattern_id, 0);

    for _ in 0..per_pattern {
        vm.tick_audio_players();
    }
    assert_eq!(vm.music_player().pattern_id, 1);
    assert_eq!(vm.music_player().song_step, 1);

    // Falling off the end of the order table jumps back to the loop point.
    for _ in 0..per_pattern {
        vm.tick_audio_players();
    }
    assert_eq!(vm.music_player().pattern_id, 0);
    assert_eq!(vm.music_player().song_step, 0);

    // Well past two full cycles, playback is still running and still in step.
    for _ in 0..per_pattern * 5 {
        vm.tick_audio_players();
    }
    assert!(
        vm.music_player().active && vm.music_player().song_active,
        "a looping song never stops on its own"
    );
    assert_eq!(vm.music_player().pattern_id, 1);
}

#[test]
fn play_music_cancels_song_mode_and_falls_back_to_one_pattern() {
    let mut vm = vm_playing_song(&[1, 2], 1);
    let per_pattern = ticks_per_pattern(&vm);

    vm.start_music(3);
    assert!(!vm.music_player().song_active);

    for _ in 0..per_pattern {
        vm.tick_audio_players();
    }
    assert!(vm.music_player().active, "single-pattern playback loops");
    assert_eq!(
        vm.music_player().pattern_id,
        3,
        "the order table must not steer playback once song mode is off"
    );
}

#[test]
fn an_empty_song_is_a_silent_no_op() {
    let vm = vm_playing_song(&[], 0);
    assert!(!vm.music_player().active);
    assert!(!vm.music_player().song_active);
}

/// A music bank holding only the 512 bytes of pattern data zero-pads to the
/// full bank length, so it reads as a song with nothing in it.
#[test]
fn a_pattern_only_music_bank_zero_pads_to_an_empty_song() {
    let mut vm = Vm::new(VmConfig::default());
    let input = Input::new();
    let font = Font::empty();

    let pattern_only_bank = vec![0u8; MUSIC_PATTERN_DATA_LEN];
    vm.load_cart_sections(&[CartSection {
        kind: SectionKind::MusicBanks,
        data: encode_asset_bank("patterns", &pattern_only_bank),
    }]);
    vm.load_lua_source(
        r#"
        function _update()
          if not started then
            started = true
            load_music_bank("patterns")
            play_music_song(0)
          end
        end
        "#,
        &input,
        &font,
    )
    .unwrap_or_else(|e| panic!("load_lua_source failed: {e}"));

    vm.run_frame(&input, &font);

    for offset in 0..(MUSIC_LOOP_POINT_OFFSET - MUSIC_ORDER_OFFSET + 1) {
        assert_eq!(
            vm.peek_memory(MUSIC_RAM_BASE + MUSIC_ORDER_OFFSET + offset),
            0,
            "loading a 512-byte bank must zero-fill the song section at +{offset}"
        );
    }
    assert!(
        !vm.music_player().active,
        "a bank with no order table has no song to play"
    );
}
