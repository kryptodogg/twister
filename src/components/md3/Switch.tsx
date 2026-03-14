import { useRef, useEffect } from 'react';
import '@material/web/switch/switch.js';

interface MdSwitchProps extends React.HTMLAttributes<HTMLElement> {
  selected?: boolean;
  onSelected?: (selected: boolean) => void;
  disabled?: boolean;
}

export function MdSwitch({ selected, onSelected, disabled, ...props }: MdSwitchProps) {
  const ref = useRef<any>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const handler = (e: any) => onSelected?.(e.target.selected);
    el.addEventListener('change', handler);
    return () => el.removeEventListener('change', handler);
  }, [onSelected]);

  return (
    <md-switch
      ref={ref}
      selected={selected || undefined}
      disabled={disabled || undefined}
      {...props}
    />
  );
}

declare global {
  namespace JSX {
    interface IntrinsicElements {
      'md-switch': any;
    }
  }
}
