const VoxaWaveformIcon = ({
  width,
  height,
  size,
  className,
}: {
  width?: number | string;
  height?: number | string;
  size?: number | string;
  className?: string;
}) => (
  <svg
    width={size ?? width ?? 126}
    height={size ?? height ?? 135}
    viewBox="0 0 126 135"
    className={className}
    fill="none"
    stroke="currentColor"
    xmlns="http://www.w3.org/2000/svg"
  >
    <rect x="27" y="47" width="10" height="41" rx="5" />
    <rect x="47" y="27" width="10" height="81" rx="5" />
    <rect x="67" y="15" width="10" height="105" rx="5" />
    <rect x="87" y="35" width="10" height="65" rx="5" />
  </svg>
);

export default VoxaWaveformIcon;
