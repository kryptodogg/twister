import { useRef, useEffect } from 'react';
import '@material/web/textfield/outlined-text-field.js';

interface MdTextFieldProps {
  label?: string;
  value?: string | number;
  onInput?: (value: string) => void;
  disabled?: boolean;
  type?: string;
  min?: string | number;
  max?: string | number;
  step?: string | number;
  prefixText?: string;
  suffixText?: string;
  className?: string;
  style?: React.CSSProperties;
}

export function MdTextField({ label, value, onInput, disabled, type, ...props }: MdTextFieldProps) {
  const ref = useRef<any>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const handler = (e: any) => onInput?.(e.target.value);
    el.addEventListener('input', handler);
    return () => el.removeEventListener('input', handler);
  }, [onInput]);

  return (
    <md-outlined-text-field
      ref={ref}
      label={label}
      value={value}
      type={type}
      disabled={disabled || undefined}
      {...props}
    />
  );
}

declare global {
  namespace JSX {
    interface IntrinsicElements {
      'md-outlined-text-field': any;
    }
  }
}
