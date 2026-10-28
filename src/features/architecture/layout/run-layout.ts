import ELK from "elkjs/lib/elk-api"
import type { ElkNode } from "elkjs/lib/elk-api"
import elkWorkerUrl from "elkjs/lib/elk-worker.min.js?url"

// ELK's own worker script, driven from the main thread. (WKWebView can't
// start workers from inside workers, so we don't wrap it in one of ours.)
let elk: InstanceType<typeof ELK> | null = null

/** Lays out an ELK graph off the main thread. */
export function runLayout(graph: ElkNode): Promise<ElkNode> {
  elk ??= new ELK({ workerFactory: () => new Worker(elkWorkerUrl) })
  return elk.layout(graph)
}
