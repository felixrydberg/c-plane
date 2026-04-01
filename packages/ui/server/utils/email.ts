import { Resend } from 'resend';

interface SendEmailOptions {
  to: string;
  subject: string;
  html: string;
}

const resendApiKey = process.env.NUXT_RESEND_API_KEY || process.env.RESEND_API_KEY;
const emailFrom = process.env.NUXT_EMAIL_FROM || process.env.EMAIL_FROM || 'noreply@pingu.240284308.xyz';

const resend = resendApiKey ? new Resend(resendApiKey) : null;

export const sendEmail = async ({ to, subject, html }: SendEmailOptions) => {
  if (!resend) {
    throw new Error('Resend API key is not configured');
  }

  try {
    const response = await resend.emails.send({
      from: emailFrom,
      to,
      subject,
      html,
    });

    if (response.error) {
      console.error('Email send error:', response.error);
      throw new Error(`Failed to send email: ${response.error.message}`);
    }

    return response;
  } catch (error) {
    console.error('Error sending email:', error);
    throw error;
  }
};
