const THEME_DARK = "dark";
const THEME_LIGHT = "light";

const resolveInitialTheme = () => THEME_DARK;

const applyTheme = (theme) => {
  const normalized = theme === THEME_LIGHT ? THEME_LIGHT : THEME_DARK;
  document.documentElement.dataset.theme = normalized;
  window.dispatchEvent(new CustomEvent("vizmat-theme-change", { detail: { theme: normalized } }));
  return normalized;
};

const currentTheme = () =>
  document.documentElement.dataset.theme || resolveInitialTheme();

window.vizmatGetTheme = () => currentTheme();
window.vizmatSetTheme = (theme) => applyTheme(theme);
window.vizmatToggleTheme = () =>
  applyTheme(currentTheme() === THEME_LIGHT ? THEME_DARK : THEME_LIGHT);

applyTheme(resolveInitialTheme());
