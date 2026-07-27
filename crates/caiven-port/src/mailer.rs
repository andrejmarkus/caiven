//! Outbound transactional email: verification links and password resets.
//!
//! When SMTP is not configured (local dev), emails are logged to stdout
//! instead of sent, so the flow can be exercised without a mail server.

use lettre::message::header::ContentType;
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

    async fn send(&self, to: &str, subject: &str, body: String) -> anyhow::Result<()> {
        let email = Message::builder()
            .from(self.from.parse()?)
            .to(to.parse()?)
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body)?;
        self.transport.send(email).await?;
        Ok(())
    }

    pub async fn send_verification(&self, to: &str, link: &str) -> anyhow::Result<()> {
        self.send(
            to,
            "Confirm your Caiven account",
            format!(
                "Welcome to Caiven!\n\nConfirm your email by opening this link:\n{link}\n\nThis link expires in 24 hours. If you didn't create this account, ignore this email."
            ),
        )
        .await
    }

    pub async fn send_password_reset(&self, to: &str, link: &str) -> anyhow::Result<()> {
        self.send(
            to,
            "Reset your Caiven password",
            format!(
                "Someone requested a password reset for this account.\n\nReset your password by opening this link:\n{link}\n\nThis link expires in 1 hour. If you didn't request this, ignore this email — your password won't change."
            ),
        )
        .await
    }

    /// Generic security-event notification (new sign-in, password changed,
    /// all sessions revoked, 2FA enabled/disabled, password set on an
    /// OAuth-only account).
    pub async fn send_security_alert(&self, to: &str, subject: &str, body: &str) -> anyhow::Result<()> {
        self.send(to, subject, body.to_string()).await
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
