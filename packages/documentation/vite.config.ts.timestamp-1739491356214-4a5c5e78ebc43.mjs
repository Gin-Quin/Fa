// vite.config.ts
import { defaultTheme } from "file:///Users/Lepzulnag/Code/fa/packages/documentation/node_modules/@sveltepress/theme-default/dist/index.js";
import { sveltepress } from "file:///Users/Lepzulnag/Code/fa/packages/documentation/node_modules/@sveltepress/vite/dist/index.mjs";
import { defineConfig } from "file:///Users/Lepzulnag/Code/fa/packages/documentation/node_modules/vite/dist/node/index.js";
import { readFileSync } from "node:fs";
import yaml from "file:///Users/Lepzulnag/Code/fa/packages/documentation/node_modules/js-yaml/dist/js-yaml.mjs";
var navbar = yaml.load(readFileSync("./src/navbar.yaml", "utf-8"));
var sidebar = yaml.load(readFileSync("./src/sidebar.yaml", "utf-8"));
var config = defineConfig({
  server: {
    port: 8e3
  },
  plugins: [
    sveltepress({
      theme: defaultTheme({
        navbar,
        sidebar,
        github: "https://github.com/Blackman99/sveltepress",
        logo: "/sveltepress.svg"
      }),
      siteConfig: {
        title: "Fa",
        description: "A programming language for the future."
      }
    })
  ]
});
var vite_config_default = config;
export {
  vite_config_default as default
};
//# sourceMappingURL=data:application/json;base64,ewogICJ2ZXJzaW9uIjogMywKICAic291cmNlcyI6IFsidml0ZS5jb25maWcudHMiXSwKICAic291cmNlc0NvbnRlbnQiOiBbImNvbnN0IF9fdml0ZV9pbmplY3RlZF9vcmlnaW5hbF9kaXJuYW1lID0gXCIvVXNlcnMvTGVwenVsbmFnL0NvZGUvZmEvcGFja2FnZXMvZG9jdW1lbnRhdGlvblwiO2NvbnN0IF9fdml0ZV9pbmplY3RlZF9vcmlnaW5hbF9maWxlbmFtZSA9IFwiL1VzZXJzL0xlcHp1bG5hZy9Db2RlL2ZhL3BhY2thZ2VzL2RvY3VtZW50YXRpb24vdml0ZS5jb25maWcudHNcIjtjb25zdCBfX3ZpdGVfaW5qZWN0ZWRfb3JpZ2luYWxfaW1wb3J0X21ldGFfdXJsID0gXCJmaWxlOi8vL1VzZXJzL0xlcHp1bG5hZy9Db2RlL2ZhL3BhY2thZ2VzL2RvY3VtZW50YXRpb24vdml0ZS5jb25maWcudHNcIjtpbXBvcnQgeyBkZWZhdWx0VGhlbWUgfSBmcm9tIFwiQHN2ZWx0ZXByZXNzL3RoZW1lLWRlZmF1bHRcIlxuaW1wb3J0IHsgc3ZlbHRlcHJlc3MgfSBmcm9tIFwiQHN2ZWx0ZXByZXNzL3ZpdGVcIlxuaW1wb3J0IHsgZGVmaW5lQ29uZmlnIH0gZnJvbSBcInZpdGVcIlxuaW1wb3J0IHsgcmVhZEZpbGVTeW5jIH0gZnJvbSBcIm5vZGU6ZnNcIlxuaW1wb3J0IHlhbWwgZnJvbSBcImpzLXlhbWxcIlxuaW1wb3J0IHR5cGUgeyBMaW5rSXRlbSB9IGZyb20gXCJ2aXJ0dWFsOnN2ZWx0ZXByZXNzL3RoZW1lLWRlZmF1bHRcIlxuXG5jb25zdCBuYXZiYXIgPSB5YW1sLmxvYWQocmVhZEZpbGVTeW5jKFwiLi9zcmMvbmF2YmFyLnlhbWxcIiwgXCJ1dGYtOFwiKSlcbmNvbnN0IHNpZGViYXIgPSB5YW1sLmxvYWQocmVhZEZpbGVTeW5jKFwiLi9zcmMvc2lkZWJhci55YW1sXCIsIFwidXRmLThcIikpXG5cbmNvbnN0IGNvbmZpZyA9IGRlZmluZUNvbmZpZyh7XG5cdHNlcnZlcjoge1xuXHRcdHBvcnQ6IDgwMDAsXG5cdH0sXG5cdHBsdWdpbnM6IFtcblx0XHRzdmVsdGVwcmVzcyh7XG5cdFx0XHR0aGVtZTogZGVmYXVsdFRoZW1lKHtcblx0XHRcdFx0bmF2YmFyOiBuYXZiYXIgYXMgTGlua0l0ZW1bXSxcblx0XHRcdFx0c2lkZWJhcjogc2lkZWJhciBhcyBSZWNvcmQ8c3RyaW5nLCBMaW5rSXRlbVtdPixcblx0XHRcdFx0Z2l0aHViOiBcImh0dHBzOi8vZ2l0aHViLmNvbS9CbGFja21hbjk5L3N2ZWx0ZXByZXNzXCIsXG5cdFx0XHRcdGxvZ286IFwiL3N2ZWx0ZXByZXNzLnN2Z1wiLFxuXHRcdFx0fSksXG5cdFx0XHRzaXRlQ29uZmlnOiB7XG5cdFx0XHRcdHRpdGxlOiBcIkZhXCIsXG5cdFx0XHRcdGRlc2NyaXB0aW9uOiBcIkEgcHJvZ3JhbW1pbmcgbGFuZ3VhZ2UgZm9yIHRoZSBmdXR1cmUuXCIsXG5cdFx0XHR9LFxuXHRcdH0pLFxuXHRdLFxufSlcblxuZXhwb3J0IGRlZmF1bHQgY29uZmlnXG4iXSwKICAibWFwcGluZ3MiOiAiO0FBQStULFNBQVMsb0JBQW9CO0FBQzVWLFNBQVMsbUJBQW1CO0FBQzVCLFNBQVMsb0JBQW9CO0FBQzdCLFNBQVMsb0JBQW9CO0FBQzdCLE9BQU8sVUFBVTtBQUdqQixJQUFNLFNBQVMsS0FBSyxLQUFLLGFBQWEscUJBQXFCLE9BQU8sQ0FBQztBQUNuRSxJQUFNLFVBQVUsS0FBSyxLQUFLLGFBQWEsc0JBQXNCLE9BQU8sQ0FBQztBQUVyRSxJQUFNLFNBQVMsYUFBYTtBQUFBLEVBQzNCLFFBQVE7QUFBQSxJQUNQLE1BQU07QUFBQSxFQUNQO0FBQUEsRUFDQSxTQUFTO0FBQUEsSUFDUixZQUFZO0FBQUEsTUFDWCxPQUFPLGFBQWE7QUFBQSxRQUNuQjtBQUFBLFFBQ0E7QUFBQSxRQUNBLFFBQVE7QUFBQSxRQUNSLE1BQU07QUFBQSxNQUNQLENBQUM7QUFBQSxNQUNELFlBQVk7QUFBQSxRQUNYLE9BQU87QUFBQSxRQUNQLGFBQWE7QUFBQSxNQUNkO0FBQUEsSUFDRCxDQUFDO0FBQUEsRUFDRjtBQUNELENBQUM7QUFFRCxJQUFPLHNCQUFROyIsCiAgIm5hbWVzIjogW10KfQo=
