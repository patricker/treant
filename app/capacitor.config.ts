import type { CapacitorConfig } from '@capacitor/cli';

// Treant Arcade — native shell config. The web assets are the docs site built
// with ARCADE_APP_BUILD=1 (analytics + service worker stripped) and copied into
// ./www by build-www.sh. Everything is bundled; the app never hits the network.
const config: CapacitorConfig = {
  appId: 'dev.mcts.arcade',
  appName: 'Treant Arcade',
  webDir: 'www',
  // Matches the arcade backdrop (#1c1830) so there's no white flash between the
  // splash screen and the WebView painting the first frame.
  backgroundColor: '#1c1830',
  android: {
    backgroundColor: '#1c1830',
  },
  ios: {
    backgroundColor: '#1c1830',
    // Let the arcade's own safe-area padding (env(safe-area-inset-*)) handle the
    // notch rather than Capacitor insetting the whole WebView.
    contentInset: 'never',
  },
  plugins: {
    SplashScreen: {
      backgroundColor: '#1c1830',
      launchAutoHide: true,
      launchShowDuration: 600,
      showSpinner: false,
      androidScaleType: 'CENTER_CROP',
    },
  },
};

export default config;
