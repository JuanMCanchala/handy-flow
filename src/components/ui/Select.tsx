import React from "react";
import SelectComponent from "react-select";
import CreatableSelect from "react-select/creatable";
import type {
  ActionMeta,
  Props as ReactSelectProps,
  SingleValue,
  StylesConfig,
} from "react-select";

export type SelectOption = {
  value: string;
  label: string;
  isDisabled?: boolean;
};

type BaseProps = {
  value: string | null;
  options: SelectOption[];
  placeholder?: string;
  disabled?: boolean;
  isLoading?: boolean;
  isClearable?: boolean;
  onChange: (value: string | null, action: ActionMeta<SelectOption>) => void;
  onBlur?: () => void;
  className?: string;
  formatCreateLabel?: (input: string) => string;
};

type CreatableProps = {
  isCreatable: true;
  onCreateOption: (value: string) => void;
};

type NonCreatableProps = {
  isCreatable?: false;
  onCreateOption?: never;
};

export type SelectProps = BaseProps & (CreatableProps | NonCreatableProps);

const baseBackground = "var(--color-surface)";
const hoverBackground = "var(--color-fill-hover)";
const focusBackground = "var(--color-surface)";
const neutralBorder = "var(--color-border-strong)";

const selectStyles: StylesConfig<SelectOption, false> = {
  control: (base, state) => ({
    ...base,
    minHeight: 32,
    borderRadius: 8,
    borderColor: state.isFocused ? "var(--color-border-strong)" : neutralBorder,
    boxShadow: state.isFocused ? "0 0 0 3px var(--color-focus)" : "none",
    backgroundColor: state.isFocused ? focusBackground : baseBackground,
    fontSize: "14px",
    color: "var(--color-text)",
    transition: "border-color 120ms ease, box-shadow 120ms ease",
    ":hover": {
      borderColor: "var(--color-border-strong)",
      backgroundColor: hoverBackground,
    },
  }),
  valueContainer: (base) => ({
    ...base,
    paddingInline: 10,
    paddingBlock: 4,
  }),
  input: (base) => ({
    ...base,
    color: "var(--color-text)",
  }),
  singleValue: (base) => ({
    ...base,
    color: "var(--color-text)",
  }),
  dropdownIndicator: (base, state) => ({
    ...base,
    color: state.isFocused ? "var(--color-text)" : "var(--color-text-tertiary)",
    ":hover": {
      color: "var(--color-text)",
    },
  }),
  clearIndicator: (base) => ({
    ...base,
    color: "var(--color-text-tertiary)",
    ":hover": {
      color: "var(--color-text)",
    },
  }),
  menu: (provided) => ({
    ...provided,
    zIndex: 30,
    backgroundColor: "var(--color-surface-raised)",
    color: "var(--color-text)",
    border: "none",
    boxShadow: "var(--shadow-pop)",
    borderRadius: 8,
  }),
  option: (base, state) => ({
    ...base,
    borderRadius: 6,
    backgroundColor: state.isSelected
      ? "var(--color-fill-selected)"
      : state.isFocused
        ? "var(--color-fill-hover)"
        : "transparent",
    color: "var(--color-text)",
    cursor: state.isDisabled ? "not-allowed" : base.cursor,
    opacity: state.isDisabled ? 0.4 : 1,
  }),
  placeholder: (base) => ({
    ...base,
    color: "var(--color-text-tertiary)",
  }),
};

export const Select: React.FC<SelectProps> = React.memo(
  ({
    value,
    options,
    placeholder,
    disabled,
    isLoading,
    isClearable = true,
    onChange,
    onBlur,
    className = "",
    isCreatable,
    formatCreateLabel,
    onCreateOption,
  }) => {
    const selectValue = React.useMemo(() => {
      if (!value) return null;
      const existing = options.find((option) => option.value === value);
      if (existing) return existing;
      return { value, label: value, isDisabled: false };
    }, [value, options]);

    const handleChange = (
      option: SingleValue<SelectOption>,
      action: ActionMeta<SelectOption>,
    ) => {
      onChange(option?.value ?? null, action);
    };

    const sharedProps: Partial<ReactSelectProps<SelectOption, false>> = {
      className,
      classNamePrefix: "app-select",
      value: selectValue,
      options,
      onChange: handleChange,
      placeholder,
      isDisabled: disabled,
      isLoading,
      onBlur,
      isClearable,
      styles: selectStyles,
    };

    if (isCreatable) {
      return (
        <CreatableSelect<SelectOption, false>
          {...sharedProps}
          onCreateOption={onCreateOption}
          formatCreateLabel={formatCreateLabel}
        />
      );
    }

    return <SelectComponent<SelectOption, false> {...sharedProps} />;
  },
);

Select.displayName = "Select";
