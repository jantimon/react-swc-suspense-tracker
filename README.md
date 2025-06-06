# react-swc-suspense-tracker

A development tool that helps you track which React Suspense boundary catches thrown promises, making it easier to debug your React applications that use Suspense for data fetching.

## The Problem

React Suspense uses thrown Promises to pause rendering until data is ready, but there's no public API to identify which Suspense boundary catches a suspension.
This makes debugging difficult when you have multiple nested Suspense boundaries and need to understand the flow of suspensions in your app.

## The Solution

This package provides:
- **SWC Plugin**: Automatically replaces `Suspense` imports to use a trackable version
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
        "react-swc-suspense-tracker/swc"
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

#### Using with SWC directly

Add to your `.swcrc`:

```json
{
  "jsc": {
    "experimental": {
      "plugins": [
        [
          "react-swc-suspense-tracker/swc"
        ]
      ]
    }
  }
}
```

### Debugging Suspense Boundaries

The following example shows how you can debug specific hooks that might suspend.

**Note:** By default `suspenseInfo` will allways be `null` in production mode.
To change that you have to set the `enabled` option to `true` in the SWC plugin configuration.

```tsx
import { useQuery } from 'react-query';
import { wrapHook } from 'suspense-tracker';

const useQueryWithDebug = process.env.NODE_ENV === 'production'
 ? useQuery
 : wrapHook(
  useQuery,
  (suspenseInfo, queryKey) => {
    if (!suspenseInfo) {
     console.warn(`Suspense triggered by ${queryKey} but no Suspense boundary found`);
    } else {
     console.info(`Suspense triggered by ${queryKey} for boundary: ${suspenseInfo}`);
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

### What the SWC Plugin does

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

#### `useThrowIfSuspenseMissing(skip?: boolean): void`

Throws an error in development if this component might suspend but has no Suspense boundary above it - NOOP in production builds

**Parameters:**
- `skip` (optional): If `true`, skips the check. Defaults to `false`.

**Throws:** Error with message explaining the missing Suspense boundary.

#### `wrapHook<T>(hook: T, onSuspense: (id: string | null, ...args: Parameters<T>) => void): T`

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