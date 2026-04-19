import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  docsSidebar: [
    {
      type: 'category',
      label: 'Overview',
      collapsed: false,
      items: ['index'],
    },
    {
      type: 'category',
      label: 'Guide',
      collapsed: false,
      items: [
        'getting-started',
        'installation',
        'authentication',
        'mock-server',
        'testing',
        'troubleshooting',
        'trade-mainline',
        'examples',
      ],
    },
    {
      type: 'category',
      label: 'Architecture',
      collapsed: false,
      items: ['project-structure'],
    },
    {
      type: 'category',
      label: 'API Reference',
      collapsed: false,
      items: [
        'reference/index',
        'reference/alpaca-core',
        'reference/alpaca-rest-http',
        'reference/alpaca-data',
        'reference/alpaca-trade',
        'reference/alpaca-mock',
        'reference/alpaca-time',
        'reference/alpaca-option',
        'reference/alpaca-facade',
        'reference/stocks',
        'reference/options-data',
        'reference/news',
        'reference/corporate-actions',
        'reference/account',
        'reference/account-configurations',
        'reference/activities',
        'reference/assets',
        'reference/calendar-clock',
        'reference/options-contracts',
        'reference/orders',
        'reference/portfolio-history',
        'reference/positions',
        'reference/watchlists',
      ],
    },
    {
      type: 'category',
      label: 'Coverage',
      collapsed: false,
      items: ['api-coverage', 'api-coverage/market-data', 'api-coverage/trading'],
    },
    {
      type: 'category',
      label: 'Release',
      collapsed: false,
      items: ['release-checklist'],
    },
  ],
};

export default sidebars;
