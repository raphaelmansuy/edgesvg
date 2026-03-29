// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import sitemap from '@astrojs/sitemap';
import tailwindcss from '@tailwindcss/vite';

// CI passes SITE and BASE_PATH from configure-pages outputs.
// Default to GitHub Pages origin for local builds without CI env vars.
const siteUrl = process.env.SITE || 'https://raphaelmansuy.github.io';
const basePath = process.env.BASE_PATH || '/edgesvg';
const normalizedBase = basePath.endsWith('/') ? basePath : `${basePath}/`;
const fullUrl = new URL(normalizedBase, siteUrl).toString().replace(/\/$/, '');
const ogImageUrl = `${fullUrl}/og-image.png`;

export default defineConfig({
  site: siteUrl,
  base: basePath,
  integrations: [
    sitemap({
      changefreq: 'weekly',
      priority: 0.7,
      lastmod: new Date(),
    }),
    starlight({
      title: 'EdgeSVG',
      description:
        'Production-grade raster-to-SVG vectorization engine. Rust-native. Scores every output. Python, Node.js, CLI & WebAssembly SDKs.',
      logo: {
        light: './src/assets/logo.svg',
        dark: './src/assets/logo-dark.svg',
        replacesTitle: true,
        alt: 'EdgeSVG',
      },
      favicon: '/favicon.svg',
      lastUpdated: true,
      social: [
        { icon: 'github', label: 'GitHub', href: 'https://github.com/raphaelmansuy/edgesvg' },
      ],
      editLink: {
        baseUrl: 'https://github.com/raphaelmansuy/edgesvg/edit/main/site/',
      },
      expressiveCode: {
        themes: ['dracula', 'github-light'],
        styleOverrides: {
          borderRadius: '0.5rem',
          codeFontFamily: "'JetBrains Mono', monospace",
          codeFontSize: '0.875rem',
          codeLineHeight: '1.7',
        },
      },
      customCss: [
        './src/styles/tokens.css',
        './src/styles/global.css',
      ],
      components: {
        Hero: './src/components/landing/Hero.astro',
        Footer: './src/components/landing/Footer.astro',
        Header: './src/components/landing/Header.astro',
      },
      head: [
        {
          tag: 'link',
          attrs: { rel: 'sitemap', href: `${normalizedBase}sitemap-index.xml` },
        },
        {
          tag: 'link',
          attrs: { rel: 'preconnect', href: 'https://fonts.googleapis.com' },
        },
        {
          tag: 'link',
          attrs: {
            rel: 'stylesheet',
            href: 'https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800&family=JetBrains+Mono:wght@400;500&display=swap',
          },
        },
        // OpenGraph
        { tag: 'meta', attrs: { property: 'og:type', content: 'website' } },
        { tag: 'meta', attrs: { property: 'og:site_name', content: 'EdgeSVG' } },
        { tag: 'meta', attrs: { property: 'og:image', content: ogImageUrl } },
        { tag: 'meta', attrs: { property: 'og:image:width', content: '1200' } },
        { tag: 'meta', attrs: { property: 'og:image:height', content: '630' } },
        {
          tag: 'meta',
          attrs: {
            property: 'og:description',
            content: 'Production-grade raster-to-SVG vectorization. Rust-native, scored output, zero ML.',
          },
        },
        { tag: 'meta', attrs: { property: 'og:locale', content: 'en_US' } },
        // Twitter Card
        { tag: 'meta', attrs: { name: 'twitter:card', content: 'summary_large_image' } },
        { tag: 'meta', attrs: { name: 'twitter:image', content: ogImageUrl } },
        {
          tag: 'meta',
          attrs: {
            name: 'twitter:description',
            content: 'Production-grade SVG vectorization. Rust-native, zero ML.',
          },
        },
        // SEO
        { tag: 'meta', attrs: { name: 'author', content: 'Raphael Mansuy' } },
        {
          tag: 'meta',
          attrs: {
            name: 'keywords',
            content: 'SVG vectorization, raster to SVG, PNG to SVG, image tracing, Rust SVG, Python SVG, Node.js SVG, WebAssembly SVG, edgesvg, open source, logo vectorization',
          },
        },
        { tag: 'meta', attrs: { name: 'robots', content: 'index, follow' } },
        { tag: 'meta', attrs: { property: 'og:url', content: fullUrl } },
        // JSON-LD: SoftwareApplication
        {
          tag: 'script',
          attrs: { type: 'application/ld+json' },
          content: JSON.stringify({
            '@context': 'https://schema.org',
            '@type': 'SoftwareApplication',
            name: 'EdgeSVG',
            description:
              'Production-grade raster-to-SVG vectorization engine written in Rust. Scores every output with SSIM, edge similarity, IoU, and more.',
            applicationCategory: 'DeveloperApplication',
            operatingSystem: 'macOS, Linux, Windows',
            offers: { '@type': 'Offer', price: '0', priceCurrency: 'USD' },
            author: { '@type': 'Person', name: 'Raphael Mansuy', url: 'https://github.com/raphaelmansuy' },
            url: fullUrl,
            downloadUrl: 'https://pypi.org/project/edgesvg/',
            softwareVersion: '0.2',
            license: 'https://opensource.org/licenses/Apache-2.0',
            programmingLanguage: ['Rust', 'Python', 'TypeScript'],
            image: ogImageUrl,
          }),
        },
      ],
      sidebar: [
        {
          label: 'Getting Started',
          items: [
            { label: 'Python', slug: 'getting-started/quick-start-python' },
            { label: 'Node.js', slug: 'getting-started/quick-start-nodejs' },
            { label: 'CLI', slug: 'getting-started/quick-start-cli' },
            { label: 'Rust', slug: 'getting-started/quick-start-rust' },
            { label: 'WebAssembly', slug: 'getting-started/quick-start-wasm' },
          ],
        },
        {
          label: 'Core Concepts',
          items: [
            { label: 'Algorithm', slug: 'concepts/algorithm' },
            { label: 'Quality Metrics', slug: 'concepts/quality-metrics' },
            { label: 'Vectorization Modes', slug: 'concepts/modes' },
          ],
        },
        {
          label: 'API Reference',
          items: [
            { label: 'Rust API', slug: 'api/rust' },
            { label: 'Python SDK', slug: 'api/python' },
            { label: 'Node.js SDK', slug: 'api/nodejs' },
            { label: 'CLI Reference', slug: 'api/cli' },
            { label: 'WASM SDK', slug: 'api/wasm' },
          ],
        },
        {
          label: 'Guides',
          items: [
            { label: 'Batch Conversion', slug: 'guides/batch-conversion' },
            { label: 'Live WASM Demo ↗', link: `${normalizedBase}demo/`, attrs: { target: '_blank', rel: 'noopener' } },
            { label: 'CI/CD & Publishing', slug: 'guides/cicd' },
          ],
        },
        { label: 'Changelog', slug: 'changelog' },
      ],
    }),
  ],
  vite: {
    plugins: [tailwindcss()],
  },
});
