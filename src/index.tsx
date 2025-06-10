import { useContext } from "react";
import { SuspenseContext } from "./internal";

/**
 * Returns information about the nearest Suspense boundary above this component
 *
 * If not set manually the format is file.tsx:line
 *
 * Returns null if no Suspense boundary is found.
 */
export const useSuspenseOwner = (): string | null =>
  useContext(SuspenseContext);

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
 * with the current Suspense boundary information.
 *
 * @param hook - The hook to wrap.
 * @param onSuspense - Function to call when a Suspense error occurs.
 * @returns A wrapped version of the hook
 *
 *
 * Usage:
 * ```tsx
 * import { useQuery } from 'react-query';
 * import { wrapHook } from 'suspense-tracker';
 *
 * const useQueryWithDebug = wrapHook(
 *   useQuery,
 *   (suspenseInfo, queryKey) => {
 *     if (!suspenseInfo) {
 *      console.warn(`Suspense boundary missing for query: ${queryKey}`);
 *     } else {
 *      console.info(`Suspense from query: ${queryKey} triggered Suspense in ${suspenseInfo}`);
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
export const wrapHook = <T extends (...args: any) => any>(
  hook: T,
  /** Called if the hook suspends */
  onSuspense: (...args: [string | null, ...NoInfer<Parameters<T>>]) => void,
): T => {
  const wrappedHook = function (...args: any[]) {
    const suspenseInfo = useSuspenseOwner();
    try {
      return hook(...args);
    } catch (error) {
      if (
        error &&
        ((typeof error === "object" && "then" in error) ||
          (error instanceof Error &&
            error.message.startsWith("Suspense Exception")))
      ) {
        onSuspense(
          suspenseInfo,
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
