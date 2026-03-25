/**
 * NDE-OS Component Registry
 *
 * Maps catalog component names to Svelte 5 render functions.
 * These are the actual implementations that @json-render/svelte's
 * <Renderer> uses to turn JSON specs into live UI.
 */

import { defineRegistry } from "@json-render/svelte";
import { catalog } from "./catalog";

import Card from "./components/Card.svelte";
import Stack from "./components/Stack.svelte";
import Grid from "./components/Grid.svelte";
import Divider from "./components/Divider.svelte";

import Heading from "./components/Heading.svelte";
import Text from "./components/Text.svelte";
import Code from "./components/Code.svelte";
import Badge from "./components/Badge.svelte";

import Metric from "./components/Metric.svelte";
import Table from "./components/Table.svelte";
import List from "./components/List.svelte";
import Progress from "./components/Progress.svelte";
import StatusDot from "./components/StatusDot.svelte";

import Button from "./components/Button.svelte";
import Input from "./components/Input.svelte";
import Toggle from "./components/Toggle.svelte";
import Select from "./components/Select.svelte";

import Alert from "./components/Alert.svelte";
import Spinner from "./components/Spinner.svelte";
import Empty from "./components/Empty.svelte";

import AppTile from "./components/AppTile.svelte";
import Terminal from "./components/Terminal.svelte";

export const { registry } = defineRegistry(catalog, {
  components: {
    Card, Stack, Grid, Divider,
    Heading, Text, Code, Badge,
    Metric, Table, List, Progress, StatusDot,
    Button, Input, Toggle, Select,
    Alert, Spinner, Empty,
    AppTile, Terminal,
  },
});
