const config = {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        primary: "#D0BCFF",
        secondary: "#80D8E1",
        surface: "#1C1B1F",
      },
      backdropBlur: {
        '3xl': '64px',
      }
    },
  },
  plugins: [],
}
export default config;
