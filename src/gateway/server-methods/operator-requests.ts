import type { OperatorRequestManager } from "../operator-request-manager.js";
import {
  ErrorCodes,
  errorShape,
  formatValidationErrors,
  validateOperatorRequestResolveParams,
  validateOperatorRequestWaitParams,
} from "../protocol/index.js";
import type { GatewayRequestHandlers } from "./types.js";

export function createOperatorRequestHandlers(
  manager: OperatorRequestManager,
): GatewayRequestHandlers {
  return {
    "operator.request.wait": async ({ params, respond }) => {
      if (!validateOperatorRequestWaitParams(params)) {
        respond(
          false,
          undefined,
          errorShape(
            ErrorCodes.INVALID_REQUEST,
            `invalid operator.request.wait params: ${formatValidationErrors(
              validateOperatorRequestWaitParams.errors,
            )}`,
          ),
        );
        return;
      }
      const id = (params as { id: string }).id.trim();
      const decisionPromise = manager.awaitDecision(id);
      if (!decisionPromise) {
        respond(
          false,
          undefined,
          errorShape(ErrorCodes.INVALID_REQUEST, "operator request expired or not found"),
        );
        return;
      }
      const snapshot = manager.getSnapshot(id);
      const resolution = await decisionPromise;
      respond(
        true,
        {
          id,
          resolution,
          createdAtMs: snapshot?.createdAtMs,
          expiresAtMs: snapshot?.expiresAtMs,
        },
        undefined,
      );
    },
    "operator.request.resolve": async ({ params, respond, client }) => {
      if (!validateOperatorRequestResolveParams(params)) {
        respond(
          false,
          undefined,
          errorShape(
            ErrorCodes.INVALID_REQUEST,
            `invalid operator.request.resolve params: ${formatValidationErrors(
              validateOperatorRequestResolveParams.errors,
            )}`,
          ),
        );
        return;
      }
      const payload = params as { id: string; resolution: unknown };
      const ok = manager.resolve(payload.id.trim(), payload.resolution, client?.connId ?? null);
      if (!ok) {
        respond(
          false,
          undefined,
          errorShape(ErrorCodes.INVALID_REQUEST, "operator request already resolved or not found"),
        );
        return;
      }
      respond(true, { id: payload.id.trim(), resolved: true }, undefined);
    },
  };
}
