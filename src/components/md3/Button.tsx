import '@material/web/button/filled-button.js';
import '@material/web/button/outlined-button.js';
import '@material/web/button/filled-tonal-button.js';

interface MdButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  children: React.ReactNode;
}

export function MdFilledButton({ children, ...props }: MdButtonProps) {
  return <md-filled-button {...props as any}>{children}</md-filled-button>;
}

export function MdOutlinedButton({ children, ...props }: MdButtonProps) {
  return <md-outlined-button {...props as any}>{children}</md-outlined-button>;
}

export function MdFilledTonalButton({ children, ...props }: MdButtonProps) {
  return <md-filled-tonal-button {...props as any}>{children}</md-filled-tonal-button>;
}

declare global {
  namespace JSX {
    interface IntrinsicElements {
      'md-filled-button': any;
      'md-outlined-button': any;
      'md-filled-tonal-button': any;
      'md-divider': any;
      'md-linear-progress': any;
      'md-elevated-card': any;
    }
  }
}
