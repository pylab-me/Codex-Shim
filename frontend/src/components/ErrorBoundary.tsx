import React from "react";
import {AlertCircle, RefreshCw} from "lucide-react";

interface ErrorBoundaryProps {
  children: React.ReactNode;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends React.Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = {hasError: false, error: null};
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return {hasError: true, error};
  }

  handleReset = () => {
    this.setState({hasError: false, error: null});
  };

  render() {
    if (this.state.hasError) {
      return (
        <div className="min-h-screen bg-bg flex items-center justify-center p-6">
          <div className="max-w-md w-full bg-surface border border-border-subtle rounded-xl p-6 text-center space-y-4">
            <div className="w-12 h-12 rounded-full bg-red-muted flex items-center justify-center mx-auto">
              <AlertCircle size={24} className="text-red"/>
            </div>
            <h1 className="text-[16px] font-semibold text-text">
              Something went wrong
            </h1>
            <p className="text-[13px] text-text-sec">
              The application encountered an unexpected error. Try refreshing the page.
            </p>
            {this.state.error && (
              <pre className="text-[11px] text-text-tert bg-surface-rounded p-3 rounded-md text-left overflow-auto max-h-32 font-mono">
                {this.state.error.message}
              </pre>
            )}
            <button
              onClick={this.handleReset}
              className="inline-flex items-center gap-2 px-4 py-2 rounded-md bg-accent text-white text-[13px] font-medium hover:bg-accent-muted transition-colors cursor-pointer"
            >
              <RefreshCw size={14}/>
              Try again
            </button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}