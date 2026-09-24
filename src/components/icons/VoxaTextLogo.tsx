import React from "react";

const VoxaTextLogo = ({
  width,
  height,
  className,
}: {
  width?: number;
  height?: number;
  className?: string;
}) => {
  return (
    <svg
      width={width}
      height={height}
      className={className}
      viewBox="0 0 220 64"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      {/* eslint-disable-next-line i18next/no-literal-string */}
      <text x="0" y="46" fontFamily="'Instrument Serif', 'Newsreader', Georgia, serif" fontSize="48" fill="var(--color-text, currentColor)">Voxa</text>
    </svg>
  );
};

export default VoxaTextLogo;
