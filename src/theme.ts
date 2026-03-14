import { createTheme } from '@mui/material/styles';

export const m3Theme = createTheme({
  palette: {
    mode: 'dark',
    primary: {
      main: '#D0BCFF', // M3 Dark Primary
    },
    secondary: {
      main: '#CCC2DC',
    },
    background: {
      default: 'transparent', // Support Mica
      paper: 'rgba(28, 27, 31, 0.7)', // M3 Surface Container
    },
  },
  typography: {
    fontFamily: 'Roboto, sans-serif', // Default for MD3
  },
  components: {
    MuiButton: {
      styleOverrides: {
        root: {
          borderRadius: 20, // M3 Full Rounded
        },
      },
    },
  },
});
