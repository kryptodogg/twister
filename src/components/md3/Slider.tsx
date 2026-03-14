import { useRef, useEffect } from 'react';
import '@material/web/slider/slider.js';

interface MdSliderProps {
  min: number;
  max: number;
  value: number;
  step?: number;
  onInput?: (value: number) => void;
  disabled?: boolean;
  className?: string;
  style?: React.CSSProperties;
}

export function MdSlider({ min, max, value, step, onInput, disabled, ...props }: MdSliderProps) {
  const ref = useRef<any>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const handler = (e: any) => onInput?.(Number(e.target.value));
    el.addEventListener('input', handler);
    return () => el.removeEventListener('input', handler);
  }, [onInput]);

  return (
    <md-slider
      ref={ref}
      min={min}
      max={max}
      value={value}
      step={step ?? 1}
      disabled={disabled || undefined}
      {...props}
    />
  );
}

declare global {
  namespace JSX {
    interface IntrinsicElements {
      'md-slider': any;
    }
  }
}
