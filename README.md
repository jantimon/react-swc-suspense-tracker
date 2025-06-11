# react-swc-suspense-tracker

[![npm version](https://img.shields.io/npm/v/react-swc-suspense-tracker.svg)](https://www.npmjs.com/package/react-swc-suspense-tracker)
[![CI](https://github.com/jantimon/react-swc-suspense-tracker/actions/workflows/test.yml/badge.svg)](https://github.com/jantimon/react-swc-suspense-tracker/actions/workflows/test.yml)

A development tool that helps you track which React Suspense and Error boundaries handle thrown promises and errors, making it easier to debug your React applications that use Suspense for data fetching and Error boundaries for error handling.

![Screenshot of React Dev Tools with Suspense Boundary Information](https://github.com/user-attachments/assets/8918f233-710b-44ab-b4a4-2a2b3c425855)

## The Problem

React Suspense uses thrown Promises to pause rendering until data is ready, and Error boundaries catch thrown errors to handle failures gracefully. However, there's no public API to identify which boundary catches a suspension or error.
This makes debugging difficult when you have multiple nested boundaries and need to understand the flow of suspensions and errors in your app.

## The Solution

This package provides:
- **SWC Plugin**: Automatically replaces `Suspense` and `ErrorBoundary` imports to use trackable versions
- **Development Hooks**: Utilities to detect missing boundaries and debug suspension/error flow

## Installation

```bash
npm install --save-dev react-swc-suspense-tracker
```

## Usage

### Next.js Setup

Add the plugin to your `next.config.js`:

```javascript
module.exports = {
  experimental: {
    swcPlugins: [
      [
        "react-swc-suspense-tracker/swc",
        {
          // Optional: track custom boundary components
          boundaries: [
            { component: "ErrorBoundary", from: "react-error-boundary" }
          ]
        }
      ],
    ],
  },
};
```

***Note:** The plugin is enabled by default in development mode and disabled in production builds. You can override this behavior by setting the `enabled` option.

#### Plugin Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | `boolean` | `true` on development<br> `false` in production | Enable/disable the plugin transformation |
| `boundaries` | `Array<{component: string, from: string}>` | `[]` | Additional boundary components to track (e.g., custom Error boundaries) |

#### Using with SWC directly

Add to your `.swcrc`:

```json
{
  "jsc": {
    "experimental": {
      "plugins": [
        [
          "react-swc-suspense-tracker/swc",
          {
            "boundaries": [
              { "component": "ErrorBoundary", "from": "react-error-boundary" }
            ]
          }
        ]
      ]
    }
  }
}
```

### Debugging Suspense Boundaries

The following example shows how you can debug specific hooks that might suspend.

**Note:** By default `suspenseInfo` will always be `null` in production mode.
To change that you have to set the `enabled` option to `true` in the SWC plugin configuration.

```tsx
import { useQuery } from 'react-query';
import { wrapSuspendableHook } from 'react-swc-suspense-tracker';

const useQueryWithDebug = process.env.NODE_ENV === 'production'
 ? useQuery
 : wrapSuspendableHook(
  useQuery,
  (suspenseBoundaries, queryKey) => {
    if (suspenseBoundaries.length === 0) {
     console.warn(`Suspense triggered by ${queryKey} but no Suspense boundary found`);
    } else {
     console.info(`Suspense triggered by ${queryKey} for boundary: ${suspenseBoundaries[0]}`);
    }
  }
);

function MyComponent() {
  const { data } = useQueryWithDebug('my-query-key', fetchData);
  return <div>{data}</div>;
}
```

### Throwing Errors for Missing Suspense Boundaries

If you want to ensure that your component is wrapped in a Suspense boundary, you can use the `useThrowIfSuspenseMissing` hook. 
This will throw an error in development if the component might suspend but has no Suspense boundary above it.

```javascript
import { useThrowIfSuspenseMissing } from "react-swc-suspense-tracker";

function MyComponent() {
  // This will throw an error in development if no Suspense boundary is found
  useThrowIfSuspenseMissing();

  const data = useSomeDataHook(); // This might suspend
  return <div>{data}</div>;
}
```

Result:

![Simple usage screenshot of useThrowIfSuspenseMissing](https://github.com/user-attachments/assets/a660aaaf-64e2-459d-8ab5-753b526ce63e)

### What the SWC Plugin does

The plugin automatically transforms:

```javascript
// Before transformation
import { Suspense, ErrorBoundary } from "react";
<Suspense fallback={<Loading />}>
  <ErrorBoundary fallback={<ErrorFallback />}>
    <MyComponent />
  </ErrorBoundary>
</Suspense>

// After transformation
import { BoundaryTrackerSWC } from "react-swc-suspense-tracker/context";
<BoundaryTrackerSWC Component={Suspense} fallback={<Loading />} id="my/file.tsx:123">
  <BoundaryTrackerSWC Component={ErrorBoundary} fallback={<ErrorFallback />} id="my/file.tsx:124">
    <MyComponent />
  </BoundaryTrackerSWC>
</BoundaryTrackerSWC>
```

### Custom logger

For custom logging or logging in production you can use the `useSuspenseOwner` hook to get the ID of the nearest Suspense boundary:

```javascript
import { 
  useSuspenseOwner,
} from "react-swc-suspense-tracker";
function MyComponent() {
  const suspenseOwner = useSuspenseOwner();
  
  // Log the Suspense boundary ID
  console.log("Closest Suspense boundary ID:", suspenseOwner);

  const data = useSomeDataHook(); // This might suspend
  return <div>{data}</div>;
}
```

## API Reference

### Hooks

#### `useSuspenseOwner(): string | null`

Returns the ID of the nearest Suspense boundary above this component. The ID format is `"file.tsx:line"` if set by the SWC plugin, or a custom string if set manually.

Returns `null` if no Suspense boundary is found.

#### `useBoundaryStack(): Array<[string, ComponentType]>`

Returns information about all boundary components above this component as an array of `[boundaryId, BoundaryComponent]` tuples, ordered from outermost to innermost boundary.

Returns empty array if no boundaries are found.

#### `useThrowIfSuspenseMissing(skip?: boolean): void`

Throws an error in development if this component might suspend but has no Suspense boundary above it - NOOP in production builds

**Parameters:**
- `skip` (optional): If `true`, skips the check. Defaults to `false`.

**Throws:** Error with message explaining the missing Suspense boundary.

#### `wrapSuspendableHook<T>(hook: T, onSuspense: (suspenseBoundaries: string[], ...args: Parameters<T>) => void): T`

Wraps a hook to catch Suspense errors and call the provided `onSuspense` function with the current Suspense boundary information.

**Parameters:**
- `hook`: The hook function to wrap
- `onSuspense`: Function called when the hook suspends, receives array of Suspense boundary IDs

**Returns:** A wrapped version of the hook with the same signature.

## Development vs Production

This package is designed for **development use only**. The SWC plugin should be disabled in production builds to avoid the additional runtime overhead.

## Build

Get the [Rust toolchain](https://www.rust-lang.org/learn/get-started) and the right target:

```bash
rustup target add wasm32-wasip1
npm run build
```

The compiled Wasm module will be available as `react_swc_suspense_tracker.wasm`.

## License

[MIT](LICENSE)
