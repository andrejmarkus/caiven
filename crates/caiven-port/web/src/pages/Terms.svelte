<script lang="ts">
  import LegalPage from '../components/LegalPage.svelte';
  import { link } from '../router.svelte';
</script>

<LegalPage
  title="Terms of service"
  updated="1 October 2026"
  lead="You keep the rights to everything you make. These terms set out what Port and other players may do with it, and what you agree to in return."
>
  {#snippet children(legal)}
    <p>
      These terms apply when you use Caiven Port, a free site for sharing carts for the Caiven fantasy console. It is run by
      {legal.operator_name ?? '[operator name]'} ("we"). Creating an account means you accept these terms and the
      <a href="/privacy" use:link>privacy policy</a>.
    </p>

    <h2>1. Accounts</h2>
    <ul>
      <li>You must be at least 16 to create an account, or older if the law where you live sets a higher age. Playing carts needs no account.</li>
      <li>Port is open worldwide. Don't use it where that is against the law, or if you are barred from receiving services under EU or US sanctions.</li>
      <li>One account per person. Give a working email address and keep your sign-in details safe. You are responsible for what happens under your account.</li>
      <li>Usernames that impersonate others or break the content rules may be changed or removed.</li>
    </ul>

    <h2>2. Your content</h2>
    <p>
      You keep the rights to the carts, screenshots, and text you publish. Publish only what you made yourself or have the right to share, and
      follow the licenses of any third-party code, art, or music you use.
    </p>
    <p>
      By publishing, you give us a free, worldwide, non-exclusive license to store, copy, run, display, and distribute that content on Caiven
      Port. This covers making screenshots and link previews and letting visitors play it and download it for personal use. The license ends
      when you delete the content. Copies that visitors already downloaded, remixes made under section 3, and backups (for up to 30 days)
      may remain.
    </p>

    <h2>3. Remixing</h2>
    <p>
      Remixing is opt-in for each cart. If you mark a cart remixable, every Port user may copy and change it and publish the result on Caiven
      Port. Each remix links back to your cart automatically. Turning remixing off stops new remixes; remixes that already exist stay. Remixes
      of your cart belong to their authors, who must keep the link back to your work.
    </p>

    <h2 id="content-rules">4. Content rules</h2>
    <p>Do not publish, in carts, screenshots, names, descriptions, or comments:</p>
    <ul>
      <li>anything illegal, or anything that infringes someone else's copyright, trademark, or other rights;</li>
      <li>sexual content involving minors (we report it to the authorities), or sexually explicit material of any kind. Port is for all ages;</li>
      <li>threats, harassment, hate speech, or content that encourages violence or self-harm;</li>
      <li>code that tries to escape the console's sandbox, attack players' devices, or harm the service;</li>
      <li>spam, or fake plays, ratings, or accounts;</li>
      <li>other people's personal data without their consent, or impersonation of another person.</li>
    </ul>

    <h2>5. Moderation</h2>
    <p>
      We act on reports and on what we notice ourselves. A person reviews every case. We use no automated content moderation; automated
      tools only rate-limit requests and check for bots. Depending on how serious the breach is, how often it happened, and whether it was
      deliberate, we may remove content or comments, or suspend an account temporarily or permanently. Accounts that infringe repeatedly are
      closed.
    </p>
    <p>
      When we remove your content or suspend your account, we email you the reason and the rule it breaks. If you disagree, reply within 6
      months and a person will look at the decision again. You can also go to a certified out-of-court dispute settlement body or a court.
    </p>

    <h2>6. Reporting content</h2>
    <p>
      Anyone, with or without an account, can report content they believe is illegal or breaks these terms using the
      <a href="/report" use:link>report form</a>. Include the link, the reason, and, for copyright, what work it infringes. We confirm receipt
      if you give an email, review the report promptly, and tell you what we decided. We may ignore people who repeatedly send clearly
      unfounded reports.
    </p>

    <h3 id="copyright">Copyright notices (including US DMCA)</h3>
    <p>
      To report copyright infringement, use the <a href="/report?category=copyright" use:link>report form</a> or email us with: the work you
      say is infringed; the link to the infringing content; your name, address, and email; a statement that you believe in good faith the use
      isn't authorized by the owner, its agent, or the law; and a statement, under penalty of perjury, that your notice is accurate and that
      you are the owner or authorized to act for them, signed with your full name. Notices meeting 17 U.S.C. § 512(c)(3) are handled under the
      Digital Millennium Copyright Act.
    </p>
    <p>
      If your content was removed by mistake, reply to the removal email with a counter-notice: what was removed and where it was; a
      statement, under penalty of perjury, that you believe in good faith it was removed by mistake or misidentification; your name, address,
      and email; consent to the jurisdiction of the courts where you live (or, outside the US, any court where we may be found); and your
      full name as signature. We forward it to the person who reported the content and restore the content in 10 to 14 business days unless
      they tell us they have gone to court.
    </p>

    <h2>7. The service</h2>
    <p>
      Port is free and provided as it is. We may change features, and we may suspend or end the service. Before ending it, we'll give account
      holders at least 30 days' notice to download their carts. Keep your own copies of your work; Caiven Studio stores them on your computer.
    </p>

    <h2>8. Liability</h2>
    <p>
      We are fully liable for damage caused intentionally or by gross negligence, for injury to life, body, or health, and wherever mandatory
      law makes us liable. Otherwise, as this is a free service, we are not liable for lost data, lost profits, or indirect damage, to the
      extent the law allows. Nothing in these terms limits your rights as a consumer.
    </p>

    <h2>9. Ending your use</h2>
    <p>
      Delete your account any time in <a href="/settings" use:link>Settings</a>. We may suspend or close accounts that break these terms, as
      described in section 5.
    </p>

    <h2>10. Changes to these terms</h2>
    <p>
      We'll email account holders at least 30 days before a significant change. If you don't agree, delete your account before the change
      takes effect. If you keep using your account after that date, you accept the new terms.
    </p>

    <h2>11. Law</h2>
    <p>
      Port is open worldwide and run from Slovakia (EU), so these terms are governed by Slovak law. Wherever you live, if you are a consumer,
      you keep the protection of the mandatory laws of your own country, and you may bring a claim in its courts.
    </p>

    <h2 id="contact">12. Legal notice and contact</h2>
    <ul>
      <li>Operator: {legal.operator_name ?? '[operator name]'}{#if legal.operator_address}, {legal.operator_address}{/if}</li>
      <li>Email: <a href={`mailto:${legal.contact_email ?? ''}`}>{legal.contact_email ?? '[contact email]'}</a>. This is also the single point of contact for users and for authorities under the EU Digital Services Act (Art. 11 and 12). We answer in English or Slovak.</li>
      <li>Copyright notices (DMCA): the same email.</li>
      <li>Supervisory authorities in the operator's country: Slovenská obchodná inšpekcia (consumer protection), Bajkalská 21/A, 827 99 Bratislava; Úrad na ochranu osobných údajov SR (data protection), Hraničná 12, 820 07 Bratislava.</li>
    </ul>
  {/snippet}
</LegalPage>
