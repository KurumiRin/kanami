import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";
import { initLanguage } from "./lib/i18n.svelte.js";

await initLanguage();
mount(App, { target: document.getElementById("app") });
