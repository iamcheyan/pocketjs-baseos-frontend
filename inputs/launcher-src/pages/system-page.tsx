import { createEffect, Index } from "solid-js";
import { Text, View, type NodeMirror } from "@pocketjs/framework/solid/components";
import { animate } from "@pocketjs/framework/animation";
import { ramUsage, type NetworkSnapshot, type SystemSnapshot } from "../system-status";

export interface SystemPageProps {
  pageIndex: number;
  network: NetworkSnapshot;
  snapshot: SystemSnapshot | null;
}

const INK = "#06111B";
const CYAN = "#3ECFFF";
const GREEN = "#00FF99";
const AMBER = "#FFB400";
const PALE = "#D8E8FF";
const MUTED = "#7893A8";
const LINE = "#1B3345";
const PAGES = ["OVERVIEW", "CPU", "MEMORY", "STORAGE", "NETWORK"];
const text = (value: string | null | undefined) => value || "--";
const percentText = (value: number | null | undefined) => value === null || value === undefined ? "--%" : `${value}%`;
const compactOs = (snapshot: SystemSnapshot | null) => text(snapshot?.os).replace(/\s*Linux\s*/i, " ").trim();
const compactKernel = (snapshot: SystemSnapshot | null) => text(snapshot?.kernel).replace(/^Linux\s+/i, "").replace(/\s*\(([^)]+)\)\s*$/, " $1");

function Bar(props: { value: number | null | undefined; color: string }) {
  const blocks = () => Array.from({ length: 16 }, (_, index) => index < Math.round(Math.max(0, Math.min(100, props.value ?? 0)) / 6.25));
  return (
    <View class="w-[320px] h-[8px] flex-row gap-[2px]">
      <Index each={blocks()}>
        {(filled) => <View class="flex-1 h-[8px]" style={{ bgColor: filled() ? props.color : LINE }} />}
      </Index>
    </View>
  );
}

export function SystemPage(props: SystemPageProps) {
  const snapshot = () => props.snapshot;
  const ram = () => ramUsage(props.snapshot);
  const cpu = () => snapshot()?.cpuUsagePercent ?? null;
  const storage = () => snapshot()?.storagePercent ?? null;
  let pageNode: NodeMirror | undefined;
  createEffect(() => {
    props.pageIndex;
    props.snapshot;
    props.network;
    if (!pageNode) return;
    try {
      animate(pageNode, "opacity", 0.7, { dur: 0 });
      animate(pageNode, "opacity", 1, { dur: 120, easing: "out" });
    } catch {
      // Current page remains visible if animation is unavailable.
    }
  });

  return (
    <View class="w-[360px] h-[218px] flex-col px-3 py-1" style={{ bgColor: INK }}>

      <View class="flex-1 flex-col justify-center gap-2 py-2" ref={(node: NodeMirror) => { pageNode = node; }}>
        {props.pageIndex === 0 ? (
          <View class="flex-col gap-2">
            <Text class="text-xs font-mono" style={{ textColor: PALE }}>{text(snapshot()?.model)} · {text(snapshot()?.hardware)} · {text(snapshot()?.ramVariant)}</Text>
            <View class="flex-row items-center justify-between">
              <Text class="text-xs font-bold" style={{ textColor: CYAN }}>CPU</Text>
              <Text class="text-base font-bold font-mono" style={{ textColor: PALE }}>{percentText(cpu())}</Text>
            </View>
            <Bar value={cpu()} color={CYAN} />
            <View class="flex-row items-center justify-between">
              <Text class="text-xs font-bold" style={{ textColor: GREEN }}>MEMORY</Text>
              <Text class="text-sm font-bold font-mono" style={{ textColor: PALE }}>{ram() ? `${ram()!.usedMb} / ${ram()!.totalMb} MB` : "-- / -- MB"}</Text>
            </View>
            <Bar value={ram()?.percent} color={GREEN} />
          </View>
        ) : null}

        {props.pageIndex === 1 ? (
          <View class="flex-col gap-3">
            <Text class="text-xs font-bold tracking-wide" style={{ textColor: CYAN }}>CPU UTILIZATION</Text>
            <Text class="text-lg font-bold" style={{ textColor: CYAN }}>{percentText(cpu())}</Text>
            <Bar value={cpu()} color={CYAN} />
            <View class="flex-row justify-between">
              <Text class="text-sm" style={{ textColor: PALE }}>FREQUENCY {text(snapshot()?.frequency)}</Text>
              <Text class="text-sm" style={{ textColor: AMBER }}>TEMP {text(snapshot()?.temperature)}</Text>
            </View>
          </View>
        ) : null}

        {props.pageIndex === 2 ? (
          <View class="flex-col gap-3">
            <Text class="text-xs font-bold tracking-wide" style={{ textColor: GREEN }}>MEMORY</Text>
            <Text class="text-lg font-bold" style={{ textColor: GREEN }}>{ram() ? `${ram()!.usedMb} / ${ram()!.totalMb} MB` : "-- / -- MB"}</Text>
            <Bar value={ram()?.percent} color={GREEN} />
            <View class="flex-row justify-between">
              <Text class="text-sm" style={{ textColor: PALE }}>USED {percentText(ram()?.percent)}</Text>
              <Text class="text-sm" style={{ textColor: MUTED }}>FREE {ram() ? `${ram()!.availableMb} MB` : "-- MB"}</Text>
            </View>
            <Text class="text-xs font-mono" style={{ textColor: MUTED }}>RAW {text(snapshot()?.ramAvailableKb?.toString())} / {text(snapshot()?.ramTotalKb?.toString())} KB</Text>
          </View>
        ) : null}

        {props.pageIndex === 3 ? (
          <View class="flex-col gap-3">
            <Text class="text-xs font-bold tracking-wide" style={{ textColor: AMBER }}>PERSISTENT STORAGE · /STORAGE</Text>
            <Text class="text-lg font-bold" style={{ textColor: AMBER }}>{snapshot()?.storageUsed && snapshot()?.storageTotal ? `${snapshot()!.storageUsed} / ${snapshot()!.storageTotal}` : "-- / --"}</Text>
            <Bar value={storage()} color={AMBER} />
            <View class="flex-row justify-between">
              <Text class="text-sm" style={{ textColor: PALE }}>USED {percentText(storage())}</Text>
              <Text class="text-sm" style={{ textColor: MUTED }}>AVAILABLE SPACE</Text>
            </View>
          </View>
        ) : null}

        {props.pageIndex === 4 ? (
          <View class="flex-col gap-3">
            <Text class="text-xs font-bold tracking-wide" style={{ textColor: CYAN }}>NETWORK</Text>
            <Text class="text-lg font-bold" style={{ textColor: CYAN }}>{text(props.network.address)}</Text>
            <View class="flex-row justify-between">
              <Text class="text-sm font-bold" style={{ textColor: PALE }}>{text(props.network.interface)?.toUpperCase()}</Text>
              <Text class="text-xs" style={{ textColor: props.network.connected ? GREEN : AMBER }}>{props.network.connected ? "WIFI CONNECTED" : "WIFI DISCONNECTED"}</Text>
            </View>
            <Text class="text-xs font-mono" style={{ textColor: MUTED }}>OS {compactOs(snapshot())} · KERNEL {compactKernel(snapshot())}</Text>
            <Text class="text-xs font-mono" style={{ textColor: MUTED }}>DISPLAY {text(snapshot()?.display)} · {text(snapshot()?.runtime)}</Text>
          </View>
        ) : null}
      </View>
    </View>
  );
}
