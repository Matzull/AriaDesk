// aria2 option metadata (types, categories and descriptions) and value conversions.

import data from './aria2-options.json';

export const optionDefs = data.options;
export const globalCategories = data.global;
export const taskOptions = data.task;
export const optionNames = data.names;
export const aria2Errors = data.errors;

export const def = (key) => optionDefs[key] || { type: 'string' };

/** Converts a value from aria2 to the text shown in the editor (lists are shown one per line). */
export function toDisplay(key, value) {
    const d = def(key);
    if (value == null) return '';
    if (Array.isArray(value)) return value.join('\n');
    return d.separator === ',' ? String(value).split(',').filter(Boolean).join('\n') : String(value);
}

/** Converts an edited value to what aria2 expects. */
export function toSubmit(key, value) {
    const d = def(key);
    if (d.type !== 'text' || !d.separator) return String(value);
    const items = String(value).split(/\r?\n/).map((s) => s.trim()).filter(Boolean);
    return d.submitFormat === 'array' ? items : items.join(d.separator);
}

export function validate(key, value) {
    const d = def(key);
    if (d.readonly) return true;
    if (value === '' || value == null) return !d.required;
    if (d.pattern && !new RegExp(d.pattern).test(value)) return false;
    if (d.type === 'integer' || d.type === 'float') {
        const n = Number(value);
        if (isNaN(n) || (d.type === 'integer' && !Number.isInteger(n))) return false;
        if (d.min != null && n < d.min) return false;
        if (d.max != null && n > d.max) return false;
    }
    return true;
}

/** Whether a task option can be shown / changed for a task status ('new' for the new task dialog). */
export function taskOptionAllowed(option, status, what) {
    const rule = what === 'show' ? option.canShow : option.canUpdate;
    return !rule || rule.split('|').includes(status);
}
