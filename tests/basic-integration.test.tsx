import React, { Suspense } from "react";
import { describe, it, expect } from "vitest";
import { renderToString } from "react-dom/server";
import { useSuspenseOwner } from "react-swc-suspense-tracker";

// Component that uses useSuspenseOwner to get boundary ID
function TestComponent() {
  const boundaryId = useSuspenseOwner();
  return <div data-testid="boundary-id">{boundaryId || "null"}</div>;
}

describe("Basic Integration Test", () => {
  it("should transform Suspense and provide boundary ID with filename and line number", () => {
    // This Suspense component should be transformed by the SWC plugin
    const element = (
      <Suspense fallback={<div>Loading...</div>}>
        <TestComponent />
      </Suspense>
    );

    // Render to string using React SSR
    const html = renderToString(element);

    // First verify basic structure
    expect(html).toContain('data-testid="boundary-id"');

    // Check if the boundary ID is not null (meaning SWC plugin worked)
    expect(html).not.toContain(">null<");

    // If the plugin worked, verify it contains filename and line number
    if (!html.includes(">null<")) {
      expect(html).toContain("basic-integration.test.tsx:");
      expect(html).toMatch(/basic-integration\.test\.tsx:\d+/);
    }
  });
});
