// The layout worker (E4 setting 5): one request at a time, answered with the same `id`.
import type { LayoutRequest } from "./layout-client";
import { runLayout } from "./layout";

interface Request {
  id: number;
  request: LayoutRequest;
}

// elk.bundled.js registers its own `onmessage` when it loads inside a worker; this replaces it.
// Messages come only from startLayoutWorker in this app's own bundle.
self.onmessage = (event: MessageEvent<Request>) => {
  const { id, request } = event.data;
  runLayout(request).then(
    (result) => self.postMessage({ id, result }),
    (error: unknown) => self.postMessage({ id, error: String(error) }),
  );
};
