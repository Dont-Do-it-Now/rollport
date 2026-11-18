<script lang="ts">
    import { tick } from "svelte"
    import { invoke } from "@tauri-apps/api/core"
    import ChevronDown from "@lucide/svelte/icons/chevron-down"
    import FolderOpen from "@lucide/svelte/icons/folder-open"
    import Menu from "$lib/components/Menu.svelte"
    import {
        LAYOUTS,
        PRESETS,
        keyOf,
        selectionOf,
        tell,
        when,
        type Folder,
        type MediaCounts,
        type MediaSelection,
        type MediaSupport,
        type MetadataMode,
        type MetadataSettings,
        type Range,
    } from "$lib/flow"

    // What the popover under Options holds: which dates this run takes, and
    // the two answers the folder keeps in its rollport-library.db — what it takes in and
    // what it names files. Three rows, each a menu, but the file names once
    // files here carry them; every choice applies as it is made.
    let {
        folder,
        range,
        counts,
        support,
    }: {
        folder: Extract<Folder, { folder: "known" }>
        range: Range
        counts: MediaCounts
        support: MediaSupport
    } = $props()

    const onrange = (range: Range) => tell("set_range", { range })

    /// Rust answers a refused pattern with why, in words to show as they are.
    async function onlayout(layout: string) {
        try {
            await invoke("set_layout", { layout })
        } catch (reason) {
            trouble = String(reason)
        }
    }

    const id = $props.id()

    const from = $derived(range.range === "between" ? range.from : daysAgo(7))
    const to = $derived(range.range === "between" ? range.to : daysAgo(0))

    function pickDate(value: string) {
        if (value === "between") {
            onrange({ range: "between", from, to })
            return
        }

        const preset = PRESETS.find((preset) => keyOf(preset.range) === value)

        if (preset) onrange(preset.range)
    }

    // Either date on its own is half an answer, so the other one comes along.
    function edge(which: "from" | "to", value: string) {
        if (!value) return

        onrange({
            range: "between",
            from: which === "from" ? value : from,
            to: which === "to" ? value : to,
        })
    }

    /// A day this many days back, as the calendar here reads it. toISOString()
    /// is UTC, which east of Greenwich hands back yesterday all evening.
    function daysAgo(days: number) {
        const day = new Date()
        day.setDate(day.getDate() - days)

        return [
            day.getFullYear(),
            String(day.getMonth() + 1).padStart(2, "0"),
            String(day.getDate()).padStart(2, "0"),
        ].join("-")
    }

    /// The one value no layout can have — a layout has to print {name} — so the
    /// menu can carry an answer that is not a layout yet.
    const OWN = "own"

    let field = $state<HTMLInputElement>()

    /// Choosing "a pattern of your own" is a decision in itself, and can be
    /// made while a preset is still in force. Until a pattern is typed, that
    /// preset is what names files and Rust is not told otherwise.
    let asked = $state(false)

    /// Why the pattern in the field is not one this folder can keep, if it is
    /// not. Empty is the ordinary case.
    let trouble = $state("")

    /// The named layout the folder uses, if it is one of ours.
    const preset = $derived(
        LAYOUTS.find((option) => option.layout === folder.layout),
    )

    const own = $derived(
        asked || !LAYOUTS.some((option) => option.layout === folder.layout),
    )

    async function pickLayout(value: string) {
        trouble = ""

        if (value !== OWN) {
            asked = false
            onlayout(value)
            return
        }

        asked = true
        // The field is only drawn once the answer is "own".
        await tick()
        field?.focus()
        field?.select()
    }

    function commit(next: string) {
        const pattern = next.trim()

        // Nothing typed: the preset still in force is the answer.
        if (!pattern) {
            trouble = ""
            asked = false
            return
        }

        // A refused pattern stays in the field, where the correction is made.
        onlayout(pattern)
    }

    const row = "relative flex h-10 items-center gap-3 px-4"
    // The ring sits inside the row: the panel scrolls, and clips anything
    // drawn outside it. Important, to win over the ring in design.css, which
    // is outside Tailwind's layers.
    const trigger = `${row} w-full text-left transition-colors -outline-offset-2! hover:bg-subtle focus-visible:bg-subtle data-[state=open]:bg-subtle`

    const DATES = [
        ...PRESETS.map((preset) => ({
            value: keyOf(preset.range),
            label: preset.label,
        })),
        { value: "between", label: "Between two dates" },
    ]

    const selection = $derived(selectionOf(folder.takes))
    let mediaOpen = $state(false)
    let metadataOpen = $state(false)
    type OptionKey = keyof MediaSelection

    function setOption(key: OptionKey, value: boolean) {
        tell("set_media_options", {
            options: { ...selection, [key]: value },
        })
    }

    const kinds: {
        key: OptionKey
        label: string
        count: keyof MediaCounts
        support?: keyof MediaSupport
        child?: boolean
        parent?: OptionKey
    }[] = [
        { key: "standard", label: "Standard photos", count: "standard" },
        { key: "live", label: "Live Photos", count: "live" },
        {
            key: "liveVideos",
            label: "Include video files",
            count: "liveVideos",
            child: true,
            parent: "live",
        },
        {
            key: "portrait",
            label: "Portrait photos",
            count: "portrait",
            support: "portrait",
        },
        {
            key: "portraitOriginal",
            label: "Include originals before portrait effect",
            count: "portraitOriginal",
            support: "portraitOriginal",
            child: true,
            parent: "portrait",
        },
        {
            key: "panorama",
            label: "Panoramas",
            count: "panorama",
            support: "panorama",
        },
        {
            key: "burst",
            label: "Bursts",
            count: "burst",
            support: "burst",
        },
        {
            key: "burstAll",
            label: "Include all frames",
            count: "burstAll",
            support: "burst",
            child: true,
            parent: "burst",
        },
        {
            key: "screenshots",
            label: "Screenshots",
            count: "screenshots",
            support: "screenshots",
        },
        { key: "videos", label: "Videos", count: "videos" },
        {
            key: "slowMotion",
            label: "Slow motion",
            count: "slowMotion",
            support: "slowMotion",
        },
    ]

    const unavailable = (kind: { support?: keyof MediaSupport }) =>
        kind.support !== undefined && !support[kind.support]

    const METADATA_MODES: { value: MetadataMode; label: string }[] = [
        { value: "fillMissing", label: "Fill missing (recommended)" },
        { value: "preserve", label: "Keep existing" },
    ]

    const metadataKinds: {
        key: keyof MetadataSettings
        label: string
    }[] = [
        { key: "location", label: "Location" },
        { key: "camera", label: "Camera and lens" },
        { key: "title", label: "Title and description" },
    ]

    function setMetadata(key: keyof MetadataSettings, value: MetadataMode) {
        tell("set_metadata_options", {
            metadata: { ...folder.metadata, [key]: value },
        })
    }

    const NAMES = [
        ...LAYOUTS.map((option) => ({
            value: option.layout,
            label: option.name,
            detail: option.example,
            mono: true,
        })),
        { value: OWN, label: "A pattern of your own", detail: ownDetail },
    ]

    const input =
        "h-7 min-w-0 cursor-text rounded-md border border-line bg-subtle px-2 select-text transition-colors hover:border-edge"
</script>

{#snippet ownDetail()}
    You write it, for example <span class="font-mono"
        >{"{mtime:%Y}/{name}"}</span
    >
{/snippet}

{#snippet answer(value: string, mono = false)}
    <span class="ml-auto flex min-w-0 items-center gap-2 font-medium">
        <span class="truncate {mono ? 'font-mono text-xs' : ''}">{value}</span>
        <ChevronDown size={12} class="shrink-0 text-muted" aria-hidden="true" />
    </span>
{/snippet}

<div class="divide-y divide-line">
    <!-- A row and the fields it opens are one group: no line between them. -->
    <div>
        <Menu
            value={keyOf(range)}
            items={DATES}
            onchoose={pickDate}
            label="Date taken, not saved with the folder"
            title="Not saved with the folder. Back to Any date when Rollport is next opened."
            class={trigger}
        >
            <span class="text-muted" aria-hidden="true">Date taken</span>
            <!-- "Between two dates" is how you ask for two dates; it is not what
                 the answer says once you have. -->
            {@render answer(when(range))}
        </Menu>

        {#if range.range === "between"}
            <div class="flex items-center gap-2 px-4 pb-3 text-xs tabular-nums">
                <input
                    type="date"
                    value={from}
                    max={to}
                    aria-label="First day"
                    onchange={(event) =>
                        edge("from", event.currentTarget.value)}
                    class="{input} flex-1"
                />
                <span class="shrink-0 text-muted">to</span>
                <input
                    type="date"
                    value={to}
                    min={from}
                    aria-label="Last day"
                    onchange={(event) => edge("to", event.currentTarget.value)}
                    class="{input} flex-1"
                />
            </div>
        {/if}
    </div>

    <div>
        <button
            type="button"
            class={trigger}
            aria-expanded={mediaOpen}
            aria-controls="{id}-media-types"
            onclick={() => (mediaOpen = !mediaOpen)}
        >
            <span class="text-muted">Photo types</span>
            <span class="ml-auto flex items-center gap-2 font-medium">
                <span>{mediaOpen ? "Hide" : "Choose"}</span>
                <ChevronDown
                    size={12}
                    class={mediaOpen ? "rotate-180" : ""}
                    aria-hidden="true"
                />
            </span>
        </button>

        {#if mediaOpen}
            <div
                id="{id}-media-types"
                class="divide-y divide-line py-2"
                aria-label="Media types"
            >
                {#each kinds as kind (kind.key)}
                    <label
                        class="flex min-h-9 items-center gap-3 px-4 text-sm {kind.child
                            ? 'pl-10 text-muted'
                            : ''} {unavailable(kind)
                            ? 'opacity-50'
                            : 'cursor-pointer'}"
                        title={unavailable(kind)
                            ? "Photo type information is unavailable on this iPhone"
                            : undefined}
                    >
                        <input
                            type="checkbox"
                            checked={selection[kind.key]}
                            disabled={unavailable(kind) ||
                                (kind.parent !== undefined && !selection[kind.parent])}
                            onchange={(event) =>
                                setOption(kind.key, event.currentTarget.checked)}
                            class="h-4 w-4 accent-ink"
                        />
                        <span class="min-w-0 flex-1 truncate">{kind.label}</span>
                        <span class="tabular-nums text-muted">{counts[kind.count]}</span>
                    </label>
                {/each}
                {#if !support.database}
                    <p class="px-4 pt-2 pb-1 text-xs text-muted">
                        This iPhone does not expose photo type information.
                    </p>
                {/if}
            </div>
        {/if}
    </div>

    <div>
        <button
            type="button"
            class={trigger}
            aria-expanded={metadataOpen}
            aria-controls="{id}-metadata"
            onclick={() => (metadataOpen = !metadataOpen)}
        >
            <span class="text-muted">Metadata</span>
            <span class="ml-auto flex items-center gap-2 font-medium">
                <span>{metadataOpen ? "Hide" : "Choose"}</span>
                <ChevronDown
                    size={12}
                    class={metadataOpen ? "rotate-180" : ""}
                    aria-hidden="true"
                />
            </span>
        </button>
        {#if metadataOpen}
            <div id="{id}-metadata" class="divide-y divide-line py-2">
                {#each metadataKinds as kind (kind.key)}
                    <Menu
                        value={folder.metadata[kind.key]}
                        items={METADATA_MODES}
                        onchoose={(value) => setMetadata(kind.key, value as MetadataMode)}
                        label={kind.label}
                        class={trigger}
                    >
                        <span class="text-muted">{kind.label}</span>
                        {@render answer(
                            METADATA_MODES.find(
                                (mode) => mode.value === folder.metadata[kind.key],
                            )?.label ?? "",
                        )}
                    </Menu>
                {/each}
                <p class="px-4 pt-2 pb-1 text-xs text-muted">
                    Existing file metadata is never deleted. Missing values are filled from the iPhone library.
                </p>
            </div>
        {/if}
    </div>

    <button
        type="button"
        class={`${row} w-full text-left transition-colors hover:bg-subtle focus-visible:bg-subtle`}
        onclick={() => tell("choose_destination")}
        title={`${folder.path}\nChoose another folder`}
    >
        <span class="text-muted">Destination folder</span>
        <span class="ml-auto flex min-w-0 items-center gap-2 font-medium">
            <span class="truncate">{folder.path}</span>
            <FolderOpen
                size={14}
                class="shrink-0 text-muted"
                aria-hidden="true"
            />
        </span>
    </button>

    <div>
            <Menu
                value={own ? OWN : folder.layout}
                items={NAMES}
                onchoose={pickLayout}
                label="File names, saved with this folder"
                title="Saved with this folder."
                class={trigger}
            >
                <span class="text-muted" aria-hidden="true">File names</span>
                {@render answer(
                    own ? "A pattern of your own" : (preset?.name ?? ""),
                )}
            </Menu>

            {#if own}
                <!-- The field is positioned, so its focus ring paints over the
                     highlighted row above it, which is positioned too. -->
                <div class="px-4 pb-3">
                    <input
                        bind:this={field}
                        type="text"
                        spellcheck="false"
                        autocapitalize="off"
                        autocorrect="off"
                        value={folder.layout}
                        aria-label="Naming pattern"
                        aria-invalid={trouble ? "true" : undefined}
                        aria-describedby="{id}-help"
                        oninput={() => (trouble = "")}
                        onchange={(event) => commit(event.currentTarget.value)}
                        class="{input} relative w-full font-mono text-xs"
                    />
                    <p
                        id="{id}-help"
                        class="mt-2 text-xs text-pretty {trouble
                            ? 'text-tomato-11'
                            : 'text-muted'}"
                    >
                        {#if trouble}
                            <!-- Its tokens set as code, as in the hint. -->
                            {#each trouble.split(/(\{[^}]*\})/) as part, index (index)}
                                {#if part.startsWith("{")}
                                    <span class="font-mono whitespace-nowrap"
                                        >{part}</span
                                    >
                                {:else}
                                    {part}
                                {/if}
                            {/each}
                        {:else}
                            <!-- A token never breaks at its hyphens. -->
                            <span class="font-mono whitespace-nowrap"
                                >{"{name}"}</span
                            >
                            is the iPhone’s name for the file,
                            <span class="font-mono whitespace-nowrap"
                                >{"{mtime:%Y-%m-%d}"}</span
                            >
                            is the date it was taken, and
                            <span class="font-mono">/</span> makes a folder.
                        {/if}
                    </p>
                </div>
            {/if}
        </div>
</div>
