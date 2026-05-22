const createEmailLayout = ({
  eyebrow,
  title,
  message,
  ctaLabel,
  ctaUrl,
  footer,
}: {
  eyebrow: string;
  title: string;
  message: string;
  ctaLabel: string;
  ctaUrl: string;
  footer: string;
}) => {
  return `
    <div style="background:#0b0d17;padding:32px 16px;font-family:Inter,Arial,sans-serif;color:#e5e7eb;">
      <div style="max-width:560px;margin:0 auto;background:#111426;border:1px solid rgba(255,255,255,0.08);border-radius:24px;overflow:hidden;box-shadow:0 24px 80px rgba(0,0,0,0.35);">
        <div style="padding:40px 40px 32px;background:linear-gradient(180deg,#1d2140 0%,#14182d 55%,#090a10 100%);">
          <p style="margin:0 0 12px;font-size:12px;letter-spacing:0.12em;text-transform:uppercase;color:#9ca3af;">${eyebrow}</p>
          <h1 style="margin:0 0 16px;font-size:32px;line-height:1.1;color:#ffffff;font-weight:700;">${title}</h1>
          <p style="margin:0;font-size:16px;line-height:1.7;color:#d1d5db;">${message}</p>
        </div>
        <div style="padding:32px 40px 40px;background:#111426;">
          <a href="${ctaUrl}" style="display:inline-block;padding:14px 22px;border-radius:14px;background:#5b5dff;color:#ffffff;text-decoration:none;font-weight:600;">
            ${ctaLabel}
          </a>
          <p style="margin:20px 0 0;font-size:14px;line-height:1.7;color:#9ca3af;word-break:break-all;">
            ${ctaUrl}
          </p>
          <p style="margin:24px 0 0;font-size:14px;line-height:1.7;color:#9ca3af;">
            ${footer}
          </p>
        </div>
      </div>
    </div>
  `;
};

export const createResetPasswordEmailTemplate = ({ url }: { url: string }) => ({
  subject: 'Reset your password',
  html: createEmailLayout({
    eyebrow: 'Password Reset',
    title: 'Reset your password',
    message: 'We received a request to reset the password for your account. Click the button below to choose a new password.',
    ctaLabel: 'Reset password',
    ctaUrl: url,
    footer: 'If you did not request this, you can safely ignore this email.',
  })
});

export const createVerifyEmailTemplate = ({ url }: { url: string }) => ({
  subject: 'Verify your email address',
  html: createEmailLayout({
    eyebrow: 'Email Verification',
    title: 'Verify your email address',
    message: 'Please confirm your email address to finish setting up your account and continue securely.',
    ctaLabel: 'Verify email',
    ctaUrl: url,
    footer: 'This verification link will expire in 24 hours.',
  })
});
