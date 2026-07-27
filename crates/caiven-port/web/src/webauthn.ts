// Thin wrapper over the browser's native WebAuthn JSON conveniences
// (`PublicKeyCredential.parseCreationOptionsFromJSON`/`.toJSON()`, Chrome
// 122+/Safari 18+/Firefox 122+), which marshal directly to/from the same
// shape the server (webauthn-rs) sends and expects — no base64url/ArrayBuffer
// conversion needed by hand.

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type Pk = any;

function supported(): boolean {
  const pk = PublicKeyCredential as Pk;
  return typeof pk !== 'undefined' && typeof pk.parseCreationOptionsFromJSON === 'function';
}

export function passkeysSupported(): boolean {
  return supported();
}

export async function createPasskey(optionsJson: { publicKey: Pk }): Promise<unknown> {
  if (!supported()) throw new Error('Passkeys are not supported in this browser.');
  const pk = PublicKeyCredential as Pk;
  const options = pk.parseCreationOptionsFromJSON(optionsJson.publicKey);
  const credential = (await navigator.credentials.create({ publicKey: options })) as Pk;
  if (!credential) throw new Error('Passkey creation was cancelled.');
  return credential.toJSON();
}

export async function getPasskey(optionsJson: { publicKey: Pk }): Promise<unknown> {
  if (!supported()) throw new Error('Passkeys are not supported in this browser.');
  const pk = PublicKeyCredential as Pk;
  const options = pk.parseRequestOptionsFromJSON(optionsJson.publicKey);
  const credential = (await navigator.credentials.get({ publicKey: options })) as Pk;
  if (!credential) throw new Error('Passkey sign-in was cancelled.');
  return credential.toJSON();
}
