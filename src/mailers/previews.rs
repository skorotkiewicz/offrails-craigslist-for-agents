//! Mailer previews (`/rails/mailers`): every mailer action, rendered from fake data. The dev
//! mailbox lists them under Previews and shows them in the same viewer as real mail. Never sent,
//! never stored, and they never touch the database.

pub use crate::kernel::mailer::Preview;

use crate::mailers::test_mailer;

// APP: one entry per mailer action, built from fake data (never the database).
pub const PREVIEWS: &[Preview] = &[
    Preview {
        name: "test_mailer/test_email",
        build: |s| Box::pin(async { test_mailer::test_email(&s.config, "ada@example.com") }),
    },
    Preview {
        name: "auth_mailer/new_sign_in",
        build: |s| {
            Box::pin(async {
                crate::mailers::auth_mailer::notice(
                    &s.config,
                    "ada@example.com",
                    &crate::mailers::auth_mailer::Notice::NewSignIn {
                        device: "Firefox on macOS".into(),
                        ip: Some("192.0.2.10".into()),
                    },
                    "preview-token-not-real",
                )
            })
        },
    },
    Preview {
        name: "auth_mailer/email_change",
        build: |s| {
            Box::pin(async {
                crate::mailers::auth_mailer::email_change(
                    &s.config,
                    "ada.new@example.com",
                    "preview-token-not-real",
                )
            })
        },
    },
    // samples: previews
];
