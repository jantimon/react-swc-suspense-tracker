import { Suspense } from "react";
import { SuspenseContext } from "./internal";

interface SuspenseTrackerProps extends React.ComponentProps<typeof Suspense> {
  id: string;
}

/**
 * Internal component that replaces React.Suspense via SWC plugin transformation.
 * Provides tracking context while maintaining all original Suspense functionality.
 */
export const SuspenseTrackerSWC = ({
  id,
  children,
  ...suspenseProps
}: SuspenseTrackerProps) => (
  <SuspenseContext.Provider value={id}>
    <Suspense {...suspenseProps}>{children}</Suspense>
  </SuspenseContext.Provider>
);
