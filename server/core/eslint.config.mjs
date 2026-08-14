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
    files: ["tests/**/*.mjs", "scripts/**/*.mjs"],
    languageOptions: {
      globals: {
        ...globals.node,
      },
    },
  },
];
