import { Static, Type } from "@sinclair/typebox";
import { NonEmptyString } from "./primitives.js";

export const OperatorRequestWaitParamsSchema = Type.Object(
  {
    id: NonEmptyString,
  },
  { additionalProperties: false },
);

export type OperatorRequestWaitParams = Static<typeof OperatorRequestWaitParamsSchema>;

export const OperatorRequestResolveParamsSchema = Type.Object(
  {
    id: NonEmptyString,
    resolution: Type.Unknown(),
  },
  { additionalProperties: false },
);

export type OperatorRequestResolveParams = Static<typeof OperatorRequestResolveParamsSchema>;
