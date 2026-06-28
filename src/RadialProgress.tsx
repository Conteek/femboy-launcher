const RadialProgress = ({ value = 50, size = 132, strokeWidth = 12 }) => {
    const outerStyle = {
        width: size,
        height: size,
        borderRadius: '50%',
        background: `conic-gradient(#FFC6B2 ${value * 3.6}deg, #383838 0deg)`,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        position: 'relative',
    } as const;

    const innerStyle = {
        width: size - strokeWidth * 2,
        height: size - strokeWidth * 2,
        borderRadius: '50%',
        background: '#1B1A19',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        fontSize: size * 0.2,
        fontWeight: 500,
        color: '#FFC6B2',
    };

    return (
        <div style={outerStyle}>
            <div style={innerStyle}>{value}%</div>
        </div>
    );
};

export default RadialProgress;