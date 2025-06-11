import { Suspense } from "react";
import { SuspenseContext } from "./internal";

interface BoundaryTrackerProps extends React.ComponentProps<typeof Suspense> {
  id: string;
  boundary?: React.ComponentType<any>;
}

/**
 * Internal component that replaces boundary components via SWC plugin transformation.
 * Provides tracking context while maintaining all original boundary functionality.
 */
export const BoundaryTrackerSWC = ({
  id,
  boundary: Boundary = Suspense,
  children,
  ...boundaryProps
}: BoundaryTrackerProps) => (
  <SuspenseContext.Provider value={id}>
    <Boundary {...boundaryProps}>{children}</Boundary>
  </SuspenseContext.Provider>
);
