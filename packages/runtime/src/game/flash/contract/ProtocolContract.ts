import type { DeepReadonly } from "../../../../../game/src/model";
import type * as Protocol from "../../../../../game/src/protocol";
import type { ProjectionEvent as HostProjectionEvent } from "../../../../../game/src/protocol";
import type { AssertEqual } from "../../../../../game/src/protocol-contract";
import type { ProjectionEvent } from "./Event";
import type {
  PacketDirection,
  PacketEncoding,
  ClientPacket,
  ServerPacket,
  ExtensionPacket,
  FlashPacket,
} from "./Packet";

type Assert<T extends true> = T;

export type ProtocolContracts = [
  Assert<AssertEqual<ProjectionEvent, DeepReadonly<HostProjectionEvent>>>,
  Assert<AssertEqual<PacketDirection, Protocol.PacketDirection>>,
  Assert<AssertEqual<PacketEncoding, Protocol.PacketEncoding>>,
  Assert<AssertEqual<
    ClientPacket,
    Extract<DeepReadonly<Protocol.Packet>, { readonly direction: "client" }>
  >>,
  Assert<AssertEqual<
    ServerPacket,
    Extract<DeepReadonly<Protocol.Packet>, { readonly direction: "server" }>
  >>,
  Assert<AssertEqual<
    ExtensionPacket,
    Extract<DeepReadonly<Protocol.Packet>, { readonly direction: "extension" }>
  >>,
  Assert<AssertEqual<FlashPacket, DeepReadonly<Protocol.Packet>>>,
];
