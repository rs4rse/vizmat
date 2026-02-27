import init, { start } from "./vizmat_core.js";

async function main() {
    const loader = document.getElementById("app-loader");
    const status = document.getElementById("loader-status");

    const setStatus = (text) => {
        if (status) {
            status.textContent = text;
        }
    };

    try {
        setStatus("Downloading and initializing WebAssembly...");
        await init();
        setStatus("Starting renderer...");
        if (loader) {
            loader.classList.add("hidden");
            window.setTimeout(() => loader.remove(), 260);
        }
        start();
    } catch (error) {
        console.error("Failed to start vizmat WASM:", error);
        setStatus("Failed to load. Hard refresh and try again.");
        if (loader) {
            loader.classList.remove("hidden");
        }
    }
}

main();
