import '@fontsource-variable/inter';
import '@fontsource-variable/space-grotesk';
import '@fontsource-variable/jetbrains-mono';
import './app.css';
import App from './App.svelte';
import { mount } from 'svelte';

mount(App, { target: document.getElementById('app')! });
