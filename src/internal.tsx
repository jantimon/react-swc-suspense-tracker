import { createContext } from "react";

/** Boundary information: [boundaryId, BoundaryComponent] */
export type BoundaryInfo = [string, React.ComponentType<any>];

/** For internal use only */
export const SuspenseContext = createContext<BoundaryInfo[]>([]);
