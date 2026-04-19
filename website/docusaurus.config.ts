import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

const config: Config = {
  title: 'alpaca-rust',
  tagline: 'Rust workspace for Alpaca HTTP SDKs, market-time semantics, and option-domain helpers',
  favicon: 'img/logo.svg',
  url: 'https://wmzhai.github.io',
  baseUrl: '/alpaca-rust/',
  organizationName: 'wmzhai',
  projectName: 'alpaca-rust',
  trailingSlash: false,
  onBrokenLinks: 'throw',
  markdown: {
    hooks: {
      onBrokenMarkdownLinks: 'throw',
    },
  },
  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },
  presets: [
    [
      'classic',
      {
        docs: {
          path: '../docs',
          routeBasePath: 'docs',
          sidebarPath: './sidebars.ts',
          editUrl: 'https://github.com/wmzhai/alpaca-rust/tree/master/',
        },
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
      } satisfies Preset.Options,
    ],
  ],
  plugins: [
    function disableBrokenWebpackBarPlugin() {
      return {
        name: 'disable-broken-webpackbar-plugin',
        configureWebpack(webpackConfig: {plugins?: unknown[]}) {
          if (Array.isArray(webpackConfig.plugins)) {
            webpackConfig.plugins = webpackConfig.plugins.filter(
              (plugin) => plugin?.constructor?.name !== 'WebpackBarPlugin',
            );
          }

          return {};
        },
      };
    },
  ],
  themeConfig: {
    image: 'img/social-card.svg',
    navbar: {
      title: 'alpaca-rust',
      logo: {
        alt: 'alpaca-rust logo',
        src: 'img/logo.svg',
      },
      items: [
        {to: '/docs', label: 'Docs', position: 'left'},
        {to: '/docs/reference', label: 'API Reference', position: 'left'},
        {to: '/docs/examples', label: 'Examples', position: 'left'},
        {
          label: 'docs.rs',
          position: 'right',
          items: [
            {href: 'https://docs.rs/alpaca-data', label: 'alpaca-data'},
            {href: 'https://docs.rs/alpaca-trade', label: 'alpaca-trade'},
            {href: 'https://docs.rs/alpaca-core', label: 'alpaca-core'},
            {href: 'https://docs.rs/alpaca-rest-http', label: 'alpaca-rest-http'},
            {href: 'https://docs.rs/alpaca-mock', label: 'alpaca-mock'},
            {href: 'https://docs.rs/alpaca-time', label: 'alpaca-time'},
            {href: 'https://docs.rs/alpaca-option', label: 'alpaca-option'},
            {href: 'https://docs.rs/alpaca-facade', label: 'alpaca-facade'},
          ],
        },
        {href: 'https://github.com/wmzhai/alpaca-rust', label: 'GitHub', position: 'right'},
      ],
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Docs',
          items: [
            {label: 'Getting Started', to: '/docs/getting-started'},
            {label: 'Project Structure', to: '/docs/project-structure'},
            {label: 'API Coverage', to: '/docs/api-coverage'},
          ],
        },
        {
          title: 'Reference',
          items: [
            {label: 'alpaca-data', to: '/docs/reference/alpaca-data'},
            {label: 'alpaca-trade', to: '/docs/reference/alpaca-trade'},
            {label: 'alpaca-time', to: '/docs/reference/alpaca-time'},
            {label: 'alpaca-option', to: '/docs/reference/alpaca-option'},
            {label: 'alpaca-facade', to: '/docs/reference/alpaca-facade'},
          ],
        },
        {
          title: 'docs.rs',
          items: [
            {label: 'alpaca-data', href: 'https://docs.rs/alpaca-data'},
            {label: 'alpaca-trade', href: 'https://docs.rs/alpaca-trade'},
            {label: 'alpaca-core', href: 'https://docs.rs/alpaca-core'},
            {label: 'alpaca-rest-http', href: 'https://docs.rs/alpaca-rest-http'},
            {label: 'alpaca-mock', href: 'https://docs.rs/alpaca-mock'},
            {label: 'alpaca-time', href: 'https://docs.rs/alpaca-time'},
            {label: 'alpaca-option', href: 'https://docs.rs/alpaca-option'},
            {label: 'alpaca-facade', href: 'https://docs.rs/alpaca-facade'},
          ],
        },
        {
          title: 'Workspace',
          items: [
            {label: 'Reference Index', to: '/docs/reference'},
            {label: 'Repository', href: 'https://github.com/wmzhai/alpaca-rust'},
            {label: 'GitHub Pages', href: 'https://wmzhai.github.io/alpaca-rust/'},
          ],
        },
      ],
      copyright: `Copyright ${new Date().getFullYear()} alpaca-rust contributors.`,
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
