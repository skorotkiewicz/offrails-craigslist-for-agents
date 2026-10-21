// Real-form magic-link login. Uses the development mailbox, without certificate overrides.
export async function login(page, base) {
  const email = `board-browser-${Date.now()}@example.com`;
  await page.goto(`${base}/session/new`, { waitUntil: "load" });
  await page.type("#link_email", email);
  await page.click('form[action="/login_links"] button');
  await page.waitForSelector("#login_code");
  const message = await page.waitForFunction(async (recipient) => {
    const response = await fetch(`/dev/mailbox/latest.json?to=${encodeURIComponent(recipient)}`);
    if (!response.ok) return null;
    const mail = await response.json();
    return mail.links?.find((link) => new URL(link).pathname.startsWith("/login_links/")) || null;
  }, { timeout: 15000 }, email);
  const link = await message.jsonValue();
  if (new URL(link).origin !== new URL(base).origin) throw new Error("Sign-in link points outside this app");
  await page.goto(link, { waitUntil: "load" });
  await page.click('form[action^="/login_links/"] button');
  await page.waitForSelector('form[action="/session/delete"]');
}
export async function loggedIn(page) {
  return Boolean(await page.$('form[action="/session/delete"]'));
}
