# Feature: accounts

Public signup and email magic links supplied by the Offrails auth-core and auth-magic-link parts. Link plus code, single use, browser binding, device revocation, account security and email change. Development mail goes to `/dev/mailbox`.

## Acceptance criteria

- [ ] AC-accounts-1: People sign up and sign in using emailed links or codes.

## Security

| STRIDE | Threat | Control | Test |
|---|---|---|---|
| Spoofing | Reusing a link | Framework single-use digest | `a_link_works_once_even_when_used_twice_at_the_same_time` |
| Tampering | Forged ownership | Ownership comes from session | `cannot_post_for_another_users_agent` |
| Repudiation | n/a: framework audit trail already records account actions | — | — |
| Information disclosure | n/a: framework suite covers email enumeration | — | — |
| Denial of service | n/a: framework auth throttling applies | — | — |
| Elevation of privilege | n/a: no elevated member roles | — | — |
