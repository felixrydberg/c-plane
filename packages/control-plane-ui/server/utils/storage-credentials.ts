import { createCipheriv, createDecipheriv, createHash, randomBytes } from "node:crypto";

const ALGORITHM = "aes-256-gcm";

const getEncryptionKey = () => {
  const rawKey = process.env.STORAGE_CREDENTIAL_ENCRYPTION_KEY;
  if (!rawKey || !rawKey.trim()) {
    throw createError({
      statusCode: 500,
      statusMessage: "Missing STORAGE_CREDENTIAL_ENCRYPTION_KEY",
    });
  }

  return createHash("sha256").update(rawKey).digest();
};

export const encryptCredential = (plainText: string) => {
  const key = getEncryptionKey();
  const iv = randomBytes(12);
  const cipher = createCipheriv(ALGORITHM, key, iv);

  const encrypted = Buffer.concat([cipher.update(plainText, "utf8"), cipher.final()]);
  const tag = cipher.getAuthTag();

  return [
    iv.toString("base64url"),
    tag.toString("base64url"),
    encrypted.toString("base64url"),
  ].join(".");
};

export const decryptCredential = (cipherText: string) => {
  const [ivPart, tagPart, payloadPart] = cipherText.split(".");
  if (!ivPart || !tagPart || !payloadPart) {
    throw new Error("Invalid encrypted credential format");
  }

  const key = getEncryptionKey();
  const iv = Buffer.from(ivPart, "base64url");
  const tag = Buffer.from(tagPart, "base64url");
  const payload = Buffer.from(payloadPart, "base64url");

  const decipher = createDecipheriv(ALGORITHM, key, iv);
  decipher.setAuthTag(tag);

  const decrypted = Buffer.concat([decipher.update(payload), decipher.final()]);
  return decrypted.toString("utf8");
};
