import { Suspense, useContext, useDebugValue } from "react";
import { SuspenseContext, type BoundaryInfo } from "./internal";

/**
 * Returns information about all boundary components above this component
 *
 * Returns an array of [boundaryId, BoundaryComponent] tuples,
 * ordered from outermost to innermost boundary.
 * If not set manually, boundaryId format is file.tsx:line
 *
 * Returns empty array if no boundaries are found.
 *
 * @dev In development mode, this hook provides React DevTools debug information
 * showing the boundary hierarchy as "Component Names → Stack IDs" for easier debugging.
 * This debug output is automatically stripped in production builds.
 */
export const useBoundaryStack = (): BoundaryInfo[] => {
  const boundaryStack = useContext(SuspenseContext);

  if (process.env.NODE_ENV === "development") {
    useDebugValue(boundaryStack, (stack) =>
      stack.length > 0
        ? `Boundaries: ${stack.map(([, Component]) => {
          if (Component === Suspense) {
            return "Suspense";
          }
          return Component.name || "Anonymous";
        }).join(" → ")}`
        : "No boundaries",
    );
  }

  return boundaryStack;
};

/**
 * Returns information about the nearest boundary above this component
 *
 * If not set manually the format is file.tsx:line
 *
 * Returns null if no boundary is found.
 */
export const useSuspenseOwner = (): string | null =>
  useBoundaryStack().find(([, Component]) => Component === Suspense)?.[0] ||
  null;

/**
 * Throws if this component might suspend but has no Suspense boundary above it
 * in development mode only.
 *
 * In production mode, it does nothing
 *
 * For production use useSuspenseOwner instead which returns null if no
 * Suspense boundary is found
 */
export const useThrowIfSuspenseMissing =
  process.env.NODE_ENV !== "production"
    ? (skip = false): void => {
        const suspenseInfo = useSuspenseOwner();
        if (!skip && !suspenseInfo) {
          throw new Error(
            "This component might suspend but has no Suspense boundary above it. " +
              "Please wrap it in a <Suspense> component or use `useThrowIfSuspenseMissing(true)` to skip this check.",
          );
        }
      }
    : () => {
        /** NOOP in Production */
      };

/**
 * Wraps a hook to catch Suspense errors and call the provided onSuspense function
 * with the current boundary information.
 *
 * @param hook - The hook to wrap.
 * @param onSuspense - Function to call when a Suspense error occurs.
 * @param onlySuspense - If true, only Suspense boundaries will be included in the stack - defaults to true
 * @returns A wrapped version of the hook
 *
 *
 * Usage:
 * ```tsx
 * import { useQuery } from 'react-query';
 * import { wrapSuspendableHook } from 'suspense-tracker';
 *
 * const useQueryWithDebug = wrapSuspendableHook(
 *   useQuery,
 *   (suspenseBoundaryStack, queryKey) => {
 *     if (suspenseBoundaryStack.length === 0) {
 *      console.warn(`No boundary found for query: ${queryKey}`);
 *     } else {
 *      console.info(`Suspense from query: ${queryKey} triggered in ${suspenseBoundaryStack[0]}`);
 *     }
 *   }
 * );
 *
 * function MyComponent() {
 *   const { data } = useQueryWithDebug('my-query-key', fetchData);
 *   return <div>{data}</div>;
 * }
 * ```
 */
export const wrapSuspendableHook = <T extends (...args: any) => any>(
  hook: T,
  /** Called if the hook suspends */
  onSuspense: (...args: [string[], ...NoInfer<Parameters<T>>]) => void,
  onlySuspense = true,
): T => {
  const wrappedHook = function (...args: any[]) {
    const boundaryStack = useBoundaryStack();
    try {
      return hook(...args);
    } catch (error) {
      const suspenseBoundaries = (
        onlySuspense
          ? boundaryStack.filter(([, Component]) => Component === Suspense)
          : boundaryStack
      ).map(([id]) => id);

      if (
        error &&
        ((typeof error === "object" && "then" in error) ||
          (error instanceof Error &&
            error.message.startsWith("Suspense Exception")))
      ) {
        onSuspense(
          suspenseBoundaries,
          // @ts-expect-error - Spread hook arguments into onSuspense
          ...args,
        );
      }
      throw error;
    }
  } as T;
  Object.defineProperty(wrappedHook, "name", {
    value: hook.name || "wrappedHook",
  });
  return wrappedHook;
};
