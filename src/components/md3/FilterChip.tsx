import { useRef, useEffect } from 'react';
import '@material/web/chips/filter-chip.js';

interface MdFilterChipProps extends React.HTMLAttributes<HTMLElement> {
  label: string;
  selected?: boolean;
  onSelected?: (selected: boolean) => void;
  disabled?: boolean;
}

export function MdFilterChip({ label, selected, onSelected, disabled, ...props }: MdFilterChipProps) {
  const ref = useRef<any>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const handler = (e: any) => onSelected?.(e.target.selected);
    el.addEventListener('change', handler);
    return () => el.removeEventListener('change', handler);
  }, [onSelected]);

  return (
    <md-filter-chip
      ref={ref}
      label={label}
      selected={selected || undefined}
      disabled={disabled || undefined}
      {...props}
    />
  );
}

declare global {
  namespace JSX {
    interface IntrinsicElements {
      'md-filter-chip': any;
    }
  }
}
