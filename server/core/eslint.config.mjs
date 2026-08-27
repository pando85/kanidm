import globals from "globals";
import pluginJs from "@eslint/js";

/** @type {import('eslint').Linter.Config[]} */
export default [
  {
    ignores: ["static/external/**", "static/rive/**"],
  },
  pluginJs.configs.recommended,
  {
    files: ["static/**/*.js", "static/**/*.mjs"],
    languageOptions: {
      globals: {
        ...globals.browser,
        Base64: "writeable", // to feed the Base64 class into the global scope
      },
    },
  },
  {
    files: ["tests/**/*.mjs"],
    languageOptions: {
      globals: {
        ...globals.node,
        // Playwright page callbacks are authored in these files but execute in the browser.
        ...globals.browser,
      },
    },
  },
  {
    files: ["scripts/**/*.mjs"],
    languageOptions: {
      globals: {
        ...globals.node,
      },
    },
  },
];
