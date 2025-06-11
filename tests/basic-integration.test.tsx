import React, { Suspense } from "react";
import { describe, it, expect } from "vitest";
import { renderToString, renderToPipeableStream } from "react-dom/server";
import {
  useSuspenseOwner as _useSuspenseOwner,
  wrapHook as _wrapSuspendableHook,
} from "react-swc-suspense-tracker";
import type * as Types from "../src/index";

const useSuspenseOwner: typeof Types.useSuspenseOwner = _useSuspenseOwner;
const wrapHook = _wrapSuspendableHook as typeof Types.wrapSuspendableHook;

describe("useSuspenseOwner()", () => {
  // Component that uses useSuspenseOwner to get boundary ID
  function TestComponent() {
    const boundaryId = useSuspenseOwner();
    return <div data-testid="boundary-id">{boundaryId || "null"}</div>;
  }

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

describe("wrapHook", () => {
  it("should call onSuspense callback when wrapped hook suspends", async () => {
    let capturedBoundaries: string[] = [];

    let didSuspend = false;
    // Test hook that throws a promise to simulate suspense
    function useSuspendingHook() {
      if (!didSuspend) {
        didSuspend = true;
        throw Promise.resolve("suspended");
      }
      return "loaded";
    }

    const wrappedHook = wrapHook(useSuspendingHook, (boundaries, ..._args) => {
      capturedBoundaries = boundaries;
    });

    function TestComponentWithWrappedHook() {
      const result = wrappedHook();
      return <div>{result}</div>;
    }

    const element = (
      <Suspense fallback={<div>Loading...</div>}>
        <TestComponentWithWrappedHook />
      </Suspense>
    );

    // Use renderToPipeableStream to properly handle Suspense
    const streamPromise = new Promise<string>((resolve, reject) => {
      const stream = renderToPipeableStream(element, {
        onShellError: (error) => {
          reject(error);
        },
        onError: (error) => {
          reject(error);
        },
        onAllReady: () => {
          // Stream completed without suspension
          resolve("Done");
        },
      });
    });

    await expect(streamPromise);

    // Verify callback was called with correct parameters
    expect(capturedBoundaries.length).toBeGreaterThan(0);
    expect(capturedBoundaries[0]).toContain("basic-integration.test.tsx");
  });
});
