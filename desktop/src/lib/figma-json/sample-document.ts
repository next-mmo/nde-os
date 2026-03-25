import type { FDocument } from "./types";

/**
 * A sample FDocument for testing the renderer.
 * Represents a modern dark-themed card UI.
 */
export const SAMPLE_DOCUMENT: FDocument = {
  version: "1.0",
  name: "Sample Card UI",
  width: 480,
  height: 360,
  background: { r: 0.06, g: 0.07, b: 0.09, a: 1 },
  meta: { generator: "manual" },
  children: [
    {
      id: "card-frame",
      name: "Card",
      type: "FRAME",
      width: 400,
      height: 280,
      layoutMode: "VERTICAL",
      primaryAxisAlignItems: "MIN",
      counterAxisAlignItems: "STRETCH",
      itemSpacing: 16,
      paddingTop: 24,
      paddingBottom: 24,
      paddingLeft: 24,
      paddingRight: 24,
      borderRadius: 16,
      fills: [
        {
          type: "SOLID",
          color: { r: 0.12, g: 0.13, b: 0.16, a: 1 },
        },
      ],
      effects: [
        {
          type: "DROP_SHADOW",
          color: { r: 0, g: 0, b: 0, a: 0.4 },
          offset: { x: 0, y: 8 },
          radius: 32,
          spread: -4,
        },
      ],
      strokes: [
        {
          color: { r: 1, g: 1, b: 1, a: 0.08 },
          weight: 1,
          align: "INSIDE",
        },
      ],
      children: [
        {
          id: "header-row",
          name: "Header Row",
          type: "FRAME",
          layoutMode: "HORIZONTAL",
          primaryAxisAlignItems: "MIN",
          counterAxisAlignItems: "CENTER",
          itemSpacing: 12,
          primaryAxisSizingMode: "HUG",
          counterAxisSizingMode: "HUG",
          children: [
            {
              id: "avatar",
              name: "Avatar",
              type: "ELLIPSE",
              width: 40,
              height: 40,
              fills: [
                {
                  type: "LINEAR_GRADIENT",
                  stops: [
                    { position: 0, color: { r: 0.4, g: 0.55, b: 1, a: 1 } },
                    { position: 1, color: { r: 0.7, g: 0.4, b: 1, a: 1 } },
                  ],
                  angle: 135,
                },
              ],
            },
            {
              id: "header-text-group",
              name: "Header Text",
              type: "FRAME",
              layoutMode: "VERTICAL",
              itemSpacing: 2,
              primaryAxisSizingMode: "HUG",
              counterAxisSizingMode: "HUG",
              children: [
                {
                  id: "title",
                  name: "Title",
                  type: "TEXT",
                  characters: "Figma JSON Renderer",
                  fontFamily: "Inter",
                  fontWeight: 600,
                  fontSize: 16,
                  lineHeight: 22,
                  fills: [
                    { type: "SOLID", color: { r: 1, g: 1, b: 1, a: 0.95 } },
                  ],
                },
                {
                  id: "subtitle",
                  name: "Subtitle",
                  type: "TEXT",
                  characters: "Render any Figma design from JSON",
                  fontFamily: "Inter",
                  fontWeight: 400,
                  fontSize: 13,
                  lineHeight: 18,
                  fills: [
                    { type: "SOLID", color: { r: 1, g: 1, b: 1, a: 0.5 } },
                  ],
                },
              ],
            },
          ],
        },
        {
          id: "divider",
          name: "Divider",
          type: "RECTANGLE",
          width: 352,
          height: 1,
          fills: [
            { type: "SOLID", color: { r: 1, g: 1, b: 1, a: 0.06 } },
          ],
        },
        {
          id: "body-text",
          name: "Body",
          type: "TEXT",
          characters:
            "This card is rendered entirely from a JSON definition. Every fill, shadow, gradient, and layout property is declaratively described and converted to CSS at runtime.",
          fontFamily: "Inter",
          fontWeight: 400,
          fontSize: 13.5,
          lineHeight: 20,
          fills: [
            { type: "SOLID", color: { r: 1, g: 1, b: 1, a: 0.7 } },
          ],
          width: 352,
        },
        {
          id: "button-row",
          name: "Button Row",
          type: "FRAME",
          layoutMode: "HORIZONTAL",
          itemSpacing: 10,
          primaryAxisSizingMode: "HUG",
          counterAxisSizingMode: "HUG",
          primaryAxisAlignItems: "MIN",
          counterAxisAlignItems: "CENTER",
          children: [
            {
              id: "btn-primary",
              name: "Primary Button",
              type: "FRAME",
              layoutMode: "HORIZONTAL",
              primaryAxisAlignItems: "CENTER",
              counterAxisAlignItems: "CENTER",
              paddingTop: 8,
              paddingBottom: 8,
              paddingLeft: 16,
              paddingRight: 16,
              borderRadius: 8,
              primaryAxisSizingMode: "HUG",
              counterAxisSizingMode: "HUG",
              fills: [
                {
                  type: "LINEAR_GRADIENT",
                  stops: [
                    { position: 0, color: { r: 0.35, g: 0.5, b: 1, a: 1 } },
                    { position: 1, color: { r: 0.55, g: 0.35, b: 1, a: 1 } },
                  ],
                  angle: 135,
                },
              ],
              onClick: "action:import",
              children: [
                {
                  id: "btn-primary-label",
                  name: "Label",
                  type: "TEXT",
                  characters: "Import Design",
                  fontFamily: "Inter",
                  fontWeight: 600,
                  fontSize: 13,
                  fills: [
                    { type: "SOLID", color: { r: 1, g: 1, b: 1, a: 1 } },
                  ],
                },
              ],
            },
            {
              id: "btn-secondary",
              name: "Secondary Button",
              type: "FRAME",
              layoutMode: "HORIZONTAL",
              primaryAxisAlignItems: "CENTER",
              counterAxisAlignItems: "CENTER",
              paddingTop: 8,
              paddingBottom: 8,
              paddingLeft: 16,
              paddingRight: 16,
              borderRadius: 8,
              primaryAxisSizingMode: "HUG",
              counterAxisSizingMode: "HUG",
              fills: [
                { type: "SOLID", color: { r: 1, g: 1, b: 1, a: 0.06 } },
              ],
              strokes: [
                {
                  color: { r: 1, g: 1, b: 1, a: 0.1 },
                  weight: 1,
                  align: "INSIDE",
                },
              ],
              onClick: "action:view-json",
              children: [
                {
                  id: "btn-secondary-label",
                  name: "Label",
                  type: "TEXT",
                  characters: "View JSON",
                  fontFamily: "Inter",
                  fontWeight: 500,
                  fontSize: 13,
                  fills: [
                    { type: "SOLID", color: { r: 1, g: 1, b: 1, a: 0.7 } },
                  ],
                },
              ],
            },
          ],
        },
      ],
    },
  ],
};
