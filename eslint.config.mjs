import globals from "globals";


/** @type {import('eslint').Linter.Config[]} */
export default [
  {ignores: [
    "docs/.vitepress/cache/**/*",
    "test/{bats,test_helper}/**/*",
    "target/**/*",
  ]},
  {languageOptions: { globals: globals.node }},
];
