import { Component, type ReactNode } from "react"
import { TriangleAlert } from "lucide-react"

import { Button } from "@/components/ui/button"

import { PaneEmpty } from "./pane"

interface Props {
  children: ReactNode
  /** Changing this resets the boundary. */
  resetKey?: unknown
}

/** Contains a crashing subtree so the rest of the app keeps working. */
export class ErrorBoundary extends Component<Props, { error: Error | null; key: unknown }> {
  state = { error: null as Error | null, key: this.props.resetKey }

  static getDerivedStateFromError(error: Error) {
    return { error }
  }

  static getDerivedStateFromProps(props: Props, state: { error: Error | null; key: unknown }) {
    return props.resetKey !== state.key ? { error: null, key: props.resetKey } : null
  }

  render() {
    if (!this.state.error) return this.props.children
    return (
      <PaneEmpty icon={<TriangleAlert />} title="Something broke here" description={this.state.error.message}>
        <Button size="sm" variant="outline" onClick={() => this.setState({ error: null })}>
          Try again
        </Button>
      </PaneEmpty>
    )
  }
}
