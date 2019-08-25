import Background from '../../../images/backgroundImage.jpg';
export const styles = theme => ({
    main: {
        marginLeft: '0',
        marginRight: '0',
        marginTop: '0',
        width: '100%',
        backgroundImage: `url(${Background})`,
        backgroundSize: 'cover',
        height: '100vh',
    },
    appBar: {
        position: 'relative',
    },
    toolbarTitle: {
        flex: 1,
    },
    errorDiv: {
        width: '100%',
        backgroundColor: '#f44842',
        marginBottom: theme.spacing.unit * 3
    },
    errorText: {
        width: '100%',
        marginBottom: theme.spacing.unit * 1,
        marginTop: theme.spacing.unit * 1,
        marginLeft: theme.spacing.unit * 1,
        marginRigth: theme.spacing.unit * 1,
        color: 'white'
    },
    layout: {
        width: 'auto',
        display: 'block',
        marginLeft: theme.spacing.unit * 3,
        marginRight: theme.spacing.unit * 3,
        [theme.breakpoints.up(400 + theme.spacing.unit * 3 * 2)]: {
            width: 400,
            marginLeft: 'auto',
            marginRight: 'auto',
        },
    },
    paper: {
        marginTop: theme.spacing.unit * 8,
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        padding: `${theme.spacing.unit * 2}px ${theme.spacing.unit * 3}px ${theme.spacing.unit * 3}px`,
    },
    avatar: {
        margin: theme.spacing.unit,
        backgroundColor: theme.palette.secondary.main,
    },
    form: {
        width: '100%',
        marginTop: theme.spacing.unit,
    },
    submit: {
        marginTop: theme.spacing.unit * 3,
    },
});
