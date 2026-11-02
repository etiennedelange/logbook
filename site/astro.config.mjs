import { defineConfig } from 'astro/config';

export default defineConfig({
  output: 'static',
  // Canonical and OG URLs. SITE_URL is a build variable on the logbook-site Worker (Workers Builds).
  site: process.env.SITE_URL ?? 'http://localhost:4321',
});
