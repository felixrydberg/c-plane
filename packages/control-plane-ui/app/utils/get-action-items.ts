import type { ContextMenuItem } from "@nuxt/ui";
import { useClipboard } from '@vueuse/core'

export const getUserActionItems = <T extends { id?: string, external?: boolean }>(seat: T, opt?: {
  onUserDelete: (user: T) => void;
  onUserExternalClick: (user: T) => void;
  onUserProjectClick: (user: T) => void;
  onUserRevokeInviteClick?: (user: T) => void;
}): ContextMenuItem[] => {
  const { copy } = useClipboard();
  const toast = useToast();

  if (!seat.id) {
    return [
      {
        type: 'label' as const,
        label: 'Actions'
      },
      {
        label: 'Revoke invite',
        color: 'error' as const,
        onSelect: () => {
          opt?.onUserRevokeInviteClick?.(seat);
        }
      }
    ];
  }

  const items = [
    {
      type: 'label' as const,
      label: 'Actions'
    },
    {
      label: 'Manage Projects',
      onSelect: () => {
        opt?.onUserProjectClick(seat);
      }
    },
    {
      label: seat.external ? 'Mark as Internal' : 'Mark as External',
      onSelect: () => {
        opt?.onUserExternalClick(seat);
      }
    },
    {
      label: 'Copy User ID',
      onSelect: () => {
        toast.add({
          title: 'Copied to clipboard',
          description: 'The user ID has been copied to your clipboard.',
          color: 'success',
        });
        copy(seat.id!);
      }
    },
    {
      label: 'Delete User',
      color: 'error' as const,
      onSelect: () => {
        opt?.onUserDelete(seat);
      }
    }
  ];

  return items;
};

export const getUserInviteActionItems = <T extends { status: string }>(invite: T, opt?: {
  onRevokeInviteClick: (invite: T) => void;
  onResendInviteClick: (invite: T) => void;
}): ContextMenuItem[] => {
  return [
    {
      type: 'label' as const,
      label: 'Actions'
    },
    {
      label: 'Resend Invitation',
      disabled: invite.status !== 'pending',
      onSelect: () => {
        opt?.onResendInviteClick(invite);
      }
    },
    {
      label: 'Revoke Invitation',
      disabled: invite.status !== 'pending',
      color: 'error',
      onSelect: () => {
        opt?.onRevokeInviteClick(invite);
      }
    }
  ];
}
