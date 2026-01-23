import { defineConfig } from '@hey-api/openapi-ts';

export default defineConfig({
  input: '../openapi.json',
  output: 'src/api',

  client: {
    type: 'fetch',
  },

  format: true,
});
