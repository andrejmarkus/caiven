<script lang="ts">
  import LegalPage from '../components/LegalPage.svelte';
  import { link } from '../router.svelte';

  const ledger = [
    ['Username, email, password hash, sign-up date', 'Running your account and sending account emails', 'Contract, Art. 6(1)(b)', 'Until you delete your account'],
    ['Sign-ins: IP address, browser user agent, times', 'Keeping your account secure: active sessions list, security log, abuse limits', 'Legitimate interest in security, Art. 6(1)(f)', 'Sessions until they expire (30 days); security log 12 months'],
    ['Two-factor secrets, passkeys, API token names (tokens are stored hashed)', 'Sign-in options you turn on', 'Contract, Art. 6(1)(b)', 'Until you remove them or delete your account'],
    ['Google, GitHub, or Discord account id and email', 'Social sign-in, only if you choose it', 'Contract, Art. 6(1)(b)', 'Until you delete your account'],
    ['Carts, screenshots, descriptions, tags, version history, comments, ratings, follows, collections, jam entries', 'Publishing and the community features you use', 'Contract, Art. 6(1)(b)', 'Until you delete them or your account (see below)'],
    ['A hashed key made from your account id or IP address when you play or remix a cart', 'Counting each player once, so play counts are honest', 'Legitimate interest, Art. 6(1)(f)', '90 days, then anonymized; only the count remains'],
    ['Content reports: your name, email, report text', 'Handling reports of illegal content, as the EU Digital Services Act requires', 'Legal obligation, Art. 6(1)(c)', 'Up to 3 years'],
    ['Moderation decisions and their reasons', 'Accountability and handling appeals', 'Legal obligation and legitimate interest', '3 years'],
    ['Server request logs: IP address, time, page', 'Defending the service against attacks', 'Legitimate interest, Art. 6(1)(f)', 'Up to 14 days'],
  ];
</script>

<LegalPage
  title="Privacy policy"
  updated="1 October 2026"
  lead="Port collects only what it needs to run accounts and share carts. No ads, no analytics, no tracking cookies, and your data is never sold."
>
  {#snippet children(legal)}
    <h2>Who is responsible</h2>
    <p>
      The controller of your data is {legal.operator_name ?? '[operator name]'}{#if legal.operator_address}, {legal.operator_address}{/if}.
      Contact: <a href={`mailto:${legal.contact_email ?? ''}`}>{legal.contact_email ?? '[contact email]'}</a>.
    </p>

    <h2>What we collect, why, and for how long</h2>
    <div class="mt-5 border-t border-[var(--border-subtle)]">
      {#each ledger as [data, why, basis, kept]}
        <div class="border-b border-[var(--border-subtle)] py-4">
          <p class="!mt-0 font-semibold text-foreground">{data}</p>
          <dl class="mt-2 grid gap-x-6 gap-y-2 text-sm sm:grid-cols-[1.4fr_1fr_1fr]">
            <div><dt class="text-xs text-muted-foreground">Why</dt><dd>{why}</dd></div>
            <div><dt class="text-xs text-muted-foreground">Legal basis</dt><dd>{basis}</dd></div>
            <div><dt class="text-xs text-muted-foreground">Kept</dt><dd>{kept}</dd></div>
          </dl>
        </div>
      {/each}
    </div>
    <p>You can play every cart without an account. An account is needed only to publish, rate, comment, or remix.</p>

    <h2>Deleting your account</h2>
    <p>
      Delete your account any time in <a href="/settings" use:link>Settings</a>. Your profile, email, sessions, sign-in methods, comments, ratings,
      follows, collections, and jam entries are deleted at once. Published carts stay playable, but your name is removed and they're credited to
      "[deleted]". To remove a cart completely, delete it before you delete your account, or ask us. Backups are overwritten within 30 days.
    </p>

    <h2>Who else receives data</h2>
    <ul>
      <li><strong>Hosting and email providers</strong> store the service and send account emails for us, under data processing agreements (Art. 28 GDPR).</li>
      <li><strong>Cloudflare Turnstile</strong> runs a bot check on sign-up, some sign-ins, password resets, and reports. Cloudflare receives your IP address and browser signals for that check.</li>
      <li><strong>Google, GitHub, Discord</strong> take part only if you sign in with them. They handle your data under their own privacy policies.</li>
      <li><strong>Have I Been Pwned</strong>: when you choose a password, we send only the first 5 characters of its SHA-1 hash, so the password can be checked against known leaks. Neither the password nor your identity is sent.</li>
      <li>Authorities, only when the law requires it.</li>
    </ul>
    <p>Cloudflare, Google, GitHub, and Discord are US companies. Transfers rely on the EU-US Data Privacy Framework or the EU Standard Contractual Clauses.</p>

    <h2>Cookies and browser storage</h2>
    <p>We use only cookies the site needs to work, so there is no cookie banner:</p>
    <ul>
      <li><code>caiven_session</code> keeps you signed in (30 days).</li>
      <li><code>caiven_csrf</code> protects your account against forged requests (30 days).</li>
      <li><code>caiven_oauth</code> exists for 10 minutes during a social sign-in.</li>
    </ul>
    <p>Your browser also stores game saves, audio settings, unpublished remix drafts, and recently played carts. That data stays on your device and is never sent to us.</p>

    <h2>Your rights, wherever you live</h2>
    <p>
      Everyone gets the same rights, whatever country you are in. You can access your data, correct it, delete it, restrict or object to its
      processing, and take it with you. Download a copy as JSON in <a href="/settings" use:link>Settings</a>, or delete your account there.
      For anything else, email us. We answer within one month and never treat you differently for using these rights.
    </p>
    <p>
      We do not sell or share your personal information for advertising, as the California Consumer Privacy Act and similar US state laws
      define those terms, and we don't use it for profiling.
    </p>
    <p>
      You can complain to the data protection authority where you live, for example your EU or EEA supervisory authority or the UK ICO. Our
      lead authority is the Office for Personal Data Protection of the Slovak Republic
      (<a href="https://dataprotection.gov.sk">dataprotection.gov.sk</a>).
    </p>

    <h2>Where your data is processed</h2>
    <p>
      Port is run from the European Union and protected by EU data protection law. Some of the providers listed above process data in the
      United States. If you use Port from another country, your data is transferred to the EU and, through those providers, to the US.
    </p>

    <h2>Children</h2>
    <p>
      You must be at least 16 to create an account, or older if the law where you live sets a higher age. Anyone can play carts without one.
      We don't knowingly collect personal data from children under 13 (as the US COPPA defines them) or under 16. If you believe a child has an
      account, tell us and we'll delete it.
    </p>

    <h2>Automated decisions</h2>
    <p>We make no decisions about you by automated means alone. Rate limits and the bot check slow down abuse; every moderation decision is made by a person.</p>

    <h2>Changes</h2>
    <p>If this policy changes in a way that matters, we'll email account holders before the change takes effect.</p>
  {/snippet}
</LegalPage>
