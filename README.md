# react-swc-suspense-tracker

A development tool that helps you track which React Suspense boundary catches thrown promises, making it easier to debug your React applications that use Suspense for data fetching.

## The Problem

React Suspense uses thrown Promises to pause rendering until data is ready, but there's no public API to identify which Suspense boundary catches a suspension.
This makes debugging difficult when you have multiple nested Suspense boundaries and need to understand the flow of suspensions in your app.

## The Solution

This package provides:
- **SWC Plugin**: Automatically replaces `Suspense` imports to use a trackable version
- **Enhanced Suspense Component**: Drop-in replacement that creates context for tracking
- **Development Hooks**: Utilities to detect missing boundaries and debug suspension flow

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
          // Only in development
          enabled: process.env.NODE_ENV === 'development'
        }
      ],
    ],
  },
};
```

### Standalone SWC Setup

Add to your `.swcrc`:

```json
{
  "jsc": {
    "experimental": {
      "plugins": [
        [
          "react-swc-suspense-tracker/swc",
          { "enabled": true }
        ]
      ]
    }
  }
}
```

### What It Does

The plugin automatically transforms:

```javascript
// Before transformation
import { Suspense } from "react";
<Suspense fallback={<Loading />}>
  <MyComponent />
</Suspense>

// After transformation
import { SuspenseTracker } from "react-swc-suspense-tracker/internal";
<SuspenseTracker fallback={<Loading />} id="my/file.tsx:123">
  <MyComponent />
</SuspenseTracker>
```

### Using the Debug Hooks

Once the plugin is active, you can use the provided hooks anywhere in your component tree:

```javascript
import { 
  useSuspenseOwner,
  wrapHook 
} from "react-swc-suspense-tracker";

function MyDataComponent() {
  // Get the ID of the nearest Suspense boundary (format: "file.tsx:line")
  const suspenseBoundaryId = useSuspenseOwner();
  if (suspenseBoundaryId) {
    console.log("Suspense Boundary ID:", suspenseBoundaryId);
  }
  
  // Your component logic that might suspend:
  const data = useSomeDataHook(); 
  
  return <div>{data}</div>;
}

function ComponentWithOptionalCheck() {
  // Skip the check by passing true
  useThrowIfSuspenseMissing(true);
  
  const data = useSomeDataHook();
  return <div>{data}</div>;
}

function MyApp() {
  return (
    <Suspense fallback={<Loading />}>
      <MyDataComponent />
      <ComponentWithOptionalCheck />
    </Suspense>
  );
}
```


### Throwing Errors for Missing Suspense Boundaries
If you want to ensure that your component is wrapped in a Suspense boundary, you can use the `useThrowIfSuspenseMissing` hook. 
This will throw an error in development if the component might suspend but has no Suspense boundary above it.

Only works if the SWC plugin is enabled otherwise it will always throw an error

```javascript
import { useThrowIfSuspenseMissing } from "react-swc-suspense-tracker";

function MyComponent() {
  // This will throw an error in development if no Suspense boundary is found
  useThrowIfSuspenseMissing();

  const data = useSomeDataHook(); // This might suspend
  return <div>{data}</div>;
}
```

### Wrapping Hooks for Suspension Debugging

You can wrap any hook to get notified when it suspends:

```javascript
import { wrapHook } from "react-swc-suspense-tracker";

const wrappedDataHook = wrapHook(
  useSomeDataHook,
  (suspenseBoundaryId) => {
    console.log("Hook suspended! Caught by boundary:", suspenseBoundaryId);
    // Log to analytics, send to monitoring service, etc.
  }
);

function MyComponent() {
  const data = wrappedDataHook(); // Will call onSuspense if it throws a promise
  return <div>{data}</div>;
}
```

## Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | `boolean` | `true` | Enable/disable the plugin transformation |

Example with all options:

```javascript
// next.config.js
module.exports = {
  experimental: {
    swcPlugins: [
      [
        "react-swc-suspense-tracker/swc",
        { 
          enabled: process.env.NODE_ENV === 'development',
        }
      ],
    ],
  },
};
```

## API Reference

### Hooks

#### `useSuspenseOwner(): string | null`

Returns the ID of the nearest Suspense boundary above this component. The ID format is `"file.tsx:line"` if set by the SWC plugin, or a custom string if set manually.

Returns `null` if no Suspense boundary is found.

#### `useThrowIfSuspenseMissing(skip?: boolean): void`

Throws an error in development if this component might suspend but has no Suspense boundary above it.

**Parameters:**
- `skip` (optional): If `true`, skips the check. Defaults to `false`.

**Throws:** Error with message explaining the missing Suspense boundary.

#### `wrapHook<T>(hook: T, onSuspense: (id: string | null) => void): T`

Wraps a hook to catch Suspense errors and call the provided `onSuspense` function with the current Suspense boundary information.

**Parameters:**
- `hook`: The hook function to wrap
- `onSuspense`: Function called when the hook suspends, receives the boundary ID

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

- [MIT license](LICENSE-MIT)