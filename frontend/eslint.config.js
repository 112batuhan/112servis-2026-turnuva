// `npm run lint`. tsc already catches unused code (noUnusedLocals); this adds the rules
// of hooks and the dependency lists of effects, memos and callbacks.
import js from "@eslint/js";
import reactHooks from "eslint-plugin-react-hooks";
import globals from "globals";
import tseslint from "typescript-eslint";

export default tseslint.config(
  { ignores: ["dist", "node_modules"] },
  {
    files: ["**/*.{ts,tsx}"],
    extends: [js.configs.recommended, ...tseslint.configs.recommended],
    languageOptions: {
      ecmaVersion: 2022,
      globals: globals.browser,
    },
    plugins: { "react-hooks": reactHooks },
    rules: {
      "react-hooks/rules-of-hooks": "error",
      "react-hooks/exhaustive-deps": "warn",
      // tsc reports these (noUnusedLocals / noUnusedParameters).
      "@typescript-eslint/no-unused-vars": "off",
    },
  },
);
