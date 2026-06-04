import { describe, expect, test } from "bun:test";
import { ConfigSchema } from "../src/config/schema.ts";
import { findCommand, KEYBIND_REGISTRY } from "../src/keybinds/registry.ts";

describe("view mode config and commands", () => {
  test("defaults older view config to day mode", () => {
    const config = ConfigSchema.parse({ view: { columns: 3 } });

    expect(config.view.columns).toBe(3);
    expect(config.view.mode).toBe("day");
  });

  test("registers month keybind and month/day commands", () => {
    const monthToggle = KEYBIND_REGISTRY.global.find((keybind) => keybind.action === "toggleMonthView");

    expect(monthToggle?.key).toBe("shift+m");
    expect(monthToggle?.display).toBe("M");
    expect(findCommand("month")?.action).toBe("setMonthView");
    expect(findCommand("day")?.action).toBe("setDayView");
  });
});
