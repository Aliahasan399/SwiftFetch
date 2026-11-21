import { useState, type FormEvent } from "react";
import { PRIORITY_MAX, PRIORITY_MIN } from "../actions";
import { Dialog } from "./Dialog";
import "./Forms.css";

export function PriorityDialog({ displayName, current, busy, onClose, onSubmit }: {
    displayName: string;
    current: number;
    busy: boolean;
    onClose: () => void;
    onSubmit: (priority: number) => void;
}) {
    const [value, setValue] = useState(String(current));
    const [error, setError] = useState<string | null>(null);

    function submit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        const raw = value.trim();
        const parsed = Number(raw);
        if (raw === "" || !Number.isSafeInteger(parsed) || parsed < PRIORITY_MIN || parsed > PRIORITY_MAX) {
            setError(`Enter a whole number from ${PRIORITY_MIN} to ${PRIORITY_MAX}.`);
            return;
        }
        onSubmit(parsed);
    }

    return <Dialog title="Set priority" description="The core confirms the change before the list updates." onClose={onClose} busy={busy}>
        <form onSubmit={(event) => { void submit(event); }}>
            <div className="dialog-body form-stack">
                {error && <p className="field-error" role="alert" id="priority-error">{error}</p>}
                <label className="field">Priority for {displayName}
                    <input data-autofocus type="number" inputMode="numeric" min={PRIORITY_MIN} max={PRIORITY_MAX} step={1} required
                        value={value} aria-invalid={Boolean(error)}
                        aria-describedby={error ? "priority-error" : "priority-help"}
                        onChange={(event) => { setValue(event.target.value); if (error) setError(null); }} />
                    <span className="field-hint" id="priority-help">Whole numbers from {PRIORITY_MIN} to {PRIORITY_MAX}. Higher numbers start first.</span>
                </label>
            </div>
            <footer className="dialog-footer"><button type="button" className="button" disabled={busy} onClick={onClose}>Cancel</button><button className="button button-primary" disabled={busy} type="submit">{busy ? "Saving…" : "Set priority"}</button></footer>
        </form>
    </Dialog>;
}
