import { MdFilterChip } from '../md3/FilterChip';

const STATUS_STYLES: Record<string, { label: string, class: string }> = {
  connected:    { label: 'CONNECTED',    class: 'text-connected border-connected/20' },
  disconnected: { label: 'DISCONNECTED', class: 'text-disconnected border-disconnected/20' },
  unwired:      { label: 'UNWIRED',      class: 'text-unwired border-unwired/20' },
};

export function StatusChip({ status }: { status: string }) {
  const cfg = STATUS_STYLES[status] || { label: 'UNKNOWN', class: '' };
  return (
    <MdFilterChip
      label={cfg.label}
      disabled={status === 'unwired'}
      className={cfg.class}
    />
  );
}
