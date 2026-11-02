import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./app";
import { LayoutContext, startLayoutWorker } from "./features/canvas/layout-client";
import "./styles.css";

// Started with the app, so elkjs is loaded before the first project opens (E4).
const layout = startLayoutWorker();

const root = document.getElementById("root");
if (root === null) {
  throw new Error("index.html has no #root element");
}
createRoot(root).render(
  <StrictMode>
    <LayoutContext value={layout}>
      <App />
    </LayoutContext>
  </StrictMode>,
);
