import { defineConfig } from 'astro/config';

export default defineConfig({
  output: 'static',
  // Set SITE_URL in the deploy workflow once the domain is decided; it feeds canonical and OG URLs.
  site: process.env.SITE_URL ?? 'http://localhost:4321',
});
