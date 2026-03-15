import { useRef, useEffect } from 'react';
import '@material/web/select/outlined-select.js';
import '@material/web/select/select-option.js';

interface MdSelectProps extends React.HTMLAttributes<HTMLElement> {
  label?: string;
  value?: string;
  onSelected?: (value: string) => void;
  disabled?: boolean;
  children: React.ReactNode;
}

export function MdSelect({ label, value, onSelected, disabled, children, ...props }: MdSelectProps) {
  const ref = useRef<any>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const handler = (e: any) => onSelected?.(e.target.value);
    el.addEventListener('change', handler);
    return () => el.removeEventListener('change', handler);
  }, [onSelected]);

  return (
    <md-outlined-select
      ref={ref}
      label={label}
      value={value}
      disabled={disabled || undefined}
      {...props}
    >
      {children}
    </md-outlined-select>
  );
}

export function MdSelectOption({ value, children, ...props }: any) {
  return (
    <md-select-option value={value} {...props}>
      <div slot="headline">{children}</div>
    </md-select-option>
  );
}

declare global {
  namespace JSX {
    interface IntrinsicElements {
      'md-outlined-select': any;
      'md-select-option': any;
    }
  }
}
