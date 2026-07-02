import {themes as prismThemes} from 'prism-react-renderer';
import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';
import remarkCodeRegion from 'remark-code-region';
import fs from 'node:fs';
import path from 'node:path';

// When packaging the site inside the native Capacitor app (`ARCADE_APP_BUILD=1`)
// we strip anything that has no business in an offline app store binary:
// Google Analytics (no network telemetry — the store listing declares "no data
// collected") and the PWA service worker (the app ships its own bundled assets;
// a second SW cache layer would only fight Capacitor's WebView).
const isAppBuild = process.env.ARCADE_APP_BUILD === '1';

const config: Config = {
  title: 'Treant',
  tagline: 'High-performance, lock-free Monte Carlo Tree Search for Rust',
  favicon: 'img/favicon.svg',

  future: {
    v4: true,
  },

  url: 'https://mcts.dev',
  baseUrl: '/',

  onBrokenLinks: 'throw',

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      'classic',
      {
        docs: {
          sidebarPath: './sidebars.ts',
          remarkPlugins: [[remarkCodeRegion, { rootDir: '..' }]],
        },
        blog: false,
        // No analytics in the native app build (offline, "no data collected").
        ...(isAppBuild
          ? {}
          : {
              gtag: {
                trackingID: 'G-GP4CNHKDF0',
                anonymizeIP: true,
              },
            }),
        theme: {
          customCss: './src/css/custom.css',
        },
      } satisfies Preset.Options,
    ],
  ],

  plugins: [
    './plugins/wasm-plugin.js',
    // The PWA service worker is omitted from the native app build: Capacitor
    // serves the bundled assets straight from the WebView, so a second SW cache
    // layer adds nothing and can shadow updates shipped in the binary.
    ...(isAppBuild
      ? []
      : [
    [
      '@docusaurus/plugin-pwa',
      {
        debug: false,
        offlineModeActivationStrategies: ['appInstalled', 'standalone', 'queryString'],
        // plugin-pwa hard-codes globPatterns (no .wasm) AFTER spreading this
        // config, so the treant engine would never be precached and offline
        // play would be dead. manifestTransforms runs at inject time, when the
        // hashed .wasm file exists in build/, so we append it there.
        injectManifestConfig: {
          manifestTransforms: [
            async (entries: {url: string; revision: string | null; size: number}[]) => {
              const buildDir = path.join(__dirname, 'build');
              const wasm = fs
                .readdirSync(buildDir)
                .filter((f) => f.endsWith('.wasm'))
                .map((f) => ({
                  url: f,
                  revision: null, // content-hashed filename is its own revision
                  size: fs.statSync(path.join(buildDir, f)).size,
                }));
              return {manifest: [...entries, ...wasm], warnings: []};
            },
          ],
        },
        pwaHead: [
          { tagName: 'link', rel: 'icon', href: '/img/icon-192.png' },
          { tagName: 'link', rel: 'manifest', href: '/manifest.json' },
          { tagName: 'meta', name: 'theme-color', content: '#1c1830' },
          { tagName: 'meta', name: 'apple-mobile-web-app-capable', content: 'yes' },
          { tagName: 'meta', name: 'apple-mobile-web-app-status-bar-style', content: 'black-translucent' },
          { tagName: 'link', rel: 'apple-touch-icon', href: '/img/icon-180.png' },
          { tagName: 'link', rel: 'mask-icon', href: '/img/favicon.svg', color: '#1c1830' },
        ],
      },
    ],
      ]),
  ],

  themeConfig: {
    colorMode: {
      respectPrefersColorScheme: true,
    },
    navbar: {
      title: 'Treant',
      logo: {
        alt: 'Treant logo',
        src: 'img/logo.svg',
        srcDark: 'img/logo-dark.svg',
      },
      items: [
        {
          type: 'docSidebar',
          sidebarId: 'docsSidebar',
          position: 'left',
          label: 'Docs',
        },
        {
          to: '/playground',
          label: 'Playground',
          position: 'left',
        },
        {
          to: '/arcade',
          label: 'Arcade',
          position: 'left',
        },
        {
          href: 'https://docs.rs/treant',
          label: 'API',
          position: 'left',
        },
        {
          href: 'https://github.com/patricker/treant',
          label: 'GitHub',
          position: 'right',
        },
      ],
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Learn',
          items: [
            {
              label: 'Docs',
              to: '/docs/intro',
            },
            {
              label: 'Playground',
              to: '/playground',
            },
          ],
        },
        {
          title: 'Reference',
          items: [
            {
              label: 'API (docs.rs)',
              href: 'https://docs.rs/treant',
            },
            {
              label: 'Crates.io',
              href: 'https://crates.io/crates/treant',
            },
          ],
        },
        {
          title: 'More',
          items: [
            {
              label: 'GitHub',
              href: 'https://github.com/patricker/treant',
            },
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} Treant Contributors. MIT License.`,
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.dracula,
      additionalLanguages: ['rust', 'toml'],
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
