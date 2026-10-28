import * as Schema from "effect/Schema";
import type * as Duration from "effect/Duration";
import type { DeepReadonly } from "@lucent/game";
import type * as Protocol from "@lucent/game/protocol";

export const PacketDirection = Schema.Literals([
  "client",
  "server",
  "extension",
]) satisfies Schema.Codec<Protocol.PacketDirection>;
export type PacketDirection = Protocol.PacketDirection;

export const PacketEncoding = Schema.Literals([
  "string",
  "json",
]) satisfies Schema.Codec<Protocol.PacketEncoding>;
export type PacketEncoding = Protocol.PacketEncoding;

export const ClientPacket = Schema.Struct({
  command: Schema.String,
  direction: Schema.Literal("client"),
  params: Schema.Array(Schema.String),
  raw: Schema.String,
  encoding: PacketEncoding,
}) satisfies Schema.Codec<ClientPacket>;
export type ClientPacket = Extract<
  FlashPacket,
  { readonly direction: "client" }
>;

export const ServerPacket = Schema.Struct({
  command: Schema.String,
  data: Schema.Unknown,
  direction: Schema.Literal("server"),
  raw: Schema.String,
  encoding: PacketEncoding,
}) satisfies Schema.Codec<ServerPacket>;
export type ServerPacket = Extract<
  FlashPacket,
  { readonly direction: "server" }
>;

export const ExtensionPacket = Schema.Struct({
  command: Schema.String,
  data: Schema.Unknown,
  direction: Schema.Literal("extension"),
  raw: Schema.String,
  encoding: PacketEncoding,
}) satisfies Schema.Codec<ExtensionPacket>;
export type ExtensionPacket = Extract<
  FlashPacket,
  { readonly direction: "extension" }
>;

export const Packet = Schema.Union([
  ClientPacket,
  ServerPacket,
  ExtensionPacket,
]) satisfies Schema.Codec<FlashPacket>;
export type FlashPacket = DeepReadonly<Protocol.Packet>;
export type Packet = FlashPacket;

export interface RawPacket {
  readonly direction: PacketDirection;
  readonly raw: string;
}

export interface PacketSelector {
  readonly command?: string;
  readonly direction?: PacketDirection;
  readonly encoding?: PacketEncoding;
  readonly predicate?: (packet: FlashPacket) => boolean;
}

export interface WaitOptions {
  /**
   * Polling interval.
   * @defaultValue "100 millis"
   */
  readonly interval?: Duration.Input;
  /**
   * Maximum wait duration.
   * @defaultValue undefined
   */
  readonly timeout?: Duration.Input;
}

export type PacketForDirection<D extends PacketDirection> = Extract<
  FlashPacket,
  { readonly direction: D }
>;

export type PacketForSelector<S extends PacketSelector | undefined> =
  S extends {
    readonly direction: infer Direction extends PacketDirection;
  }
    ? PacketForDirection<Direction>
    : FlashPacket;

export const packetData = (packet: FlashPacket): unknown =>
  packet.direction === "client" ? packet.params : packet.data;

export const matchesPacket = <S extends PacketSelector | undefined>(
  packet: FlashPacket,
  selector: S,
): packet is PacketForSelector<S> =>
  (selector?.command === undefined || selector.command === packet.command) &&
  (selector?.direction === undefined ||
    selector.direction === packet.direction) &&
  (selector?.encoding === undefined || selector.encoding === packet.encoding) &&
  (selector?.predicate === undefined || selector.predicate(packet));
