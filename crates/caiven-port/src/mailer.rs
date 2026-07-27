//! Outbound transactional email: verification links and password resets.
//!
//! When SMTP is not configured (local dev), emails are logged to stdout
//! instead of sent, so the flow can be exercised without a mail server.
//!
//! Every email is sent as `multipart/alternative` — a plain-text body for
//! text-only clients and screen readers, plus an HTML body styled to match
//! Caiven's "Obsidian & Ember" brand (see `docs/brand-colors.md`).

use lettre::message::{header::ContentType, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

#[derive(Clone)]
pub struct Mailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: String,
}

#[derive(Clone, Default)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from: String,
}

impl Mailer {
    pub fn new(cfg: &SmtpConfig) -> anyhow::Result<Self> {
        let creds = Credentials::new(cfg.username.clone(), cfg.password.clone());
        let transport = AsyncSmtpTransport::<Tokio1Executor>::relay(&cfg.host)?
            .port(cfg.port)
            .credentials(creds)
            .build();
        Ok(Mailer {
            transport,
            from: cfg.from.clone(),
        })
    }

    async fn send_multipart(
        &self,
        to: &str,
        subject: &str,
        plain: String,
        html: String,
    ) -> anyhow::Result<()> {
        let email = Message::builder()
            .from(self.from.parse()?)
            .to(to.parse()?)
            .subject(subject)
            .multipart(MultiPart::alternative().singlepart(
                SinglePart::builder().header(ContentType::TEXT_PLAIN).body(plain),
            ).singlepart(
                SinglePart::builder().header(ContentType::TEXT_HTML).body(html),
            ))?;
        self.transport.send(email).await?;
        Ok(())
    }

    pub async fn send_verification(&self, to: &str, link: &str) -> anyhow::Result<()> {
        let plain = format!(
            "Welcome to Caiven!\n\nConfirm your email by opening this link:\n{link}\n\nThis link expires in 24 hours. If you didn't create this account, ignore this email."
        );
        let html = email_shell(
            "Confirm your email",
            &format!(
                "<p style=\"{P}\">Welcome to Caiven! Confirm your email address to finish setting up your account.</p>\
                 <p style=\"{P}\">This link expires in 24 hours. If you didn't create this account, you can safely ignore this email.</p>",
                P = P_STYLE,
            ),
            Some(("Confirm email", link)),
        );
        self.send_multipart(to, "Confirm your Caiven account", plain, html).await
    }

    pub async fn send_password_reset(&self, to: &str, link: &str) -> anyhow::Result<()> {
        let plain = format!(
            "Someone requested a password reset for this account.\n\nReset your password by opening this link:\n{link}\n\nThis link expires in 1 hour. If you didn't request this, ignore this email — your password won't change."
        );
        let html = email_shell(
            "Reset your password",
            &format!(
                "<p style=\"{P}\">Someone requested a password reset for this account.</p>\
                 <p style=\"{P}\">This link expires in 1 hour. If you didn't request this, ignore this email — your password won't change.</p>",
                P = P_STYLE,
            ),
            Some(("Reset password", link)),
        );
        self.send_multipart(to, "Reset your Caiven password", plain, html).await
    }

    /// Generic security-event notification (new sign-in, password changed,
    /// all sessions revoked, 2FA enabled/disabled, password set on an
    /// OAuth-only account).
    pub async fn send_security_alert(&self, to: &str, subject: &str, body: &str) -> anyhow::Result<()> {
        let html = email_shell(subject, &paragraphs_to_html(body), None);
        self.send_multipart(to, subject, body.to_string(), html).await
    }
}

/// Sends via SMTP when a mailer is configured; otherwise logs the link to
/// stdout so the flow works in local dev without a mail server.
pub async fn send_or_log_verification(mailer: Option<&Mailer>, to: &str, link: &str) {
    match mailer {
        Some(m) => {
            if let Err(e) = m.send_verification(to, link).await {
                log::error!("failed to send verification email to {to}: {e}");
            }
        }
        None => log::info!("[dev] verification link for {to}: {link}"),
    }
}

pub async fn send_or_log_reset(mailer: Option<&Mailer>, to: &str, link: &str) {
    match mailer {
        Some(m) => {
            if let Err(e) = m.send_password_reset(to, link).await {
                log::error!("failed to send password reset email to {to}: {e}");
            }
        }
        None => log::info!("[dev] password reset link for {to}: {link}"),
    }
}

/// Best-effort security notification — never blocks the action it's
/// attached to on delivery failure.
pub async fn send_or_log_alert(mailer: Option<&Mailer>, to: &str, subject: &str, body: &str) {
    match mailer {
        Some(m) => {
            if let Err(e) = m.send_security_alert(to, subject, body).await {
                log::error!("failed to send security alert to {to}: {e}");
            }
        }
        None => log::info!("[dev] security alert for {to}: {subject} — {body}"),
    }
}

// --- Brand email shell ------------------------------------------------
//
// Inline-styled, table-based HTML so it renders consistently across mail
// clients (no external CSS/fonts/images). Colors are the "Obsidian & Ember"
// tokens from docs/brand-colors.md.

const P_STYLE: &str = "margin:0 0 16px;color:#9A9898;font-size:15px;line-height:1.6;";

/// Escapes text pulled into HTML (IP addresses, passkey labels, etc. are
/// interpolated into alert bodies upstream before reaching us).
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Splits a plain-text body on blank lines into escaped `<p>` blocks.
fn paragraphs_to_html(text: &str) -> String {
    text.split("\n\n")
        .map(|para| format!("<p style=\"{P_STYLE}\">{}</p>", escape_html(para).replace('\n', "<br>")))
        .collect::<Vec<_>>()
        .join("")
}

/// Wraps `body_html` in the branded email shell: dark card, Caiven mark,
/// heading, body, optional ember CTA button, and footer.
fn email_shell(heading: &str, body_html: &str, cta: Option<(&str, &str)>) -> String {
    let heading = escape_html(heading);
    let cta_html = match cta {
        Some((label, link)) => {
            let label = escape_html(label);
            format!(
                r#"<table role="presentation" cellpadding="0" cellspacing="0" border="0" style="margin:8px 0 24px;">
  <tr>
    <td style="border-radius:8px;background-color:#FEB05D;">
      <a href="{link}" style="display:inline-block;padding:12px 24px;font-family:Helvetica,Arial,sans-serif;font-size:15px;font-weight:600;color:#3A2308;text-decoration:none;border-radius:8px;">{label}</a>
    </td>
  </tr>
</table>
<p style="margin:0 0 24px;color:#727070;font-size:13px;line-height:1.5;">Or paste this link into your browser:<br>
  <a href="{link}" style="color:#FEB05D;word-break:break-all;">{link}</a>
</p>"#
            )
        }
        None => String::new(),
    };

    format!(
        r#"<!doctype html>
<html>
  <body style="margin:0;padding:32px 16px;background-color:#2B2A2A;font-family:Helvetica,Arial,sans-serif;">
    <table role="presentation" cellpadding="0" cellspacing="0" border="0" width="100%" style="max-width:480px;margin:0 auto;">
      <tr>
        <td style="background-color:#3F3E3E;border:1px solid #605E5E;border-radius:12px;padding:32px;">
          <table role="presentation" cellpadding="0" cellspacing="0" border="0" style="margin:0 0 24px;">
            <tr>
              <td style="width:32px;height:32px;background-color:#3B3E48;border-radius:8px;text-align:center;vertical-align:middle;font-family:Helvetica,Arial,sans-serif;font-weight:700;font-size:16px;color:#FFFFFF;border-bottom:3px solid #FEB05D;">C</td>
              <td style="padding-left:10px;font-family:Helvetica,Arial,sans-serif;font-weight:700;font-size:16px;color:#F5F2F2;">Caiven</td>
            </tr>
          </table>
          <h1 style="margin:0 0 16px;color:#F5F2F2;font-size:20px;line-height:1.3;">{heading}</h1>
          {body_html}
          {cta_html}
          <p style="margin:24px 0 0;padding-top:16px;border-top:1px solid #605E5E;color:#727070;font-size:12px;line-height:1.5;">Automated message from Caiven — please don't reply to this email.</p>
        </td>
      </tr>
    </table>
  </body>
</html>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_includes_heading_link_and_brand_color() {
        let html = email_shell(
            "Confirm your email",
            "<p>body</p>",
            Some(("Confirm email", "https://port.caiven.dev/verify-email?token=abc123")),
        );
        assert!(html.contains("Confirm your email"));
        assert!(html.contains("https://port.caiven.dev/verify-email?token=abc123"));
        assert!(html.contains("#FEB05D"));
    }

    #[test]
    fn shell_escapes_heading() {
        let html = email_shell("<script>alert(1)</script>", "<p>body</p>", None);
        assert!(!html.contains("<script>alert(1)</script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn paragraphs_split_and_escape() {
        let html = paragraphs_to_html("first para\n\nsecond <b>para</b>");
        assert!(html.contains("first para"));
        assert!(html.contains("&lt;b&gt;para&lt;/b&gt;"));
        assert_eq!(html.matches("<p").count(), 2);
    }
}
