import React from 'react';
import Typography from '@material-ui/core/Typography';
import CircularProgress from '@material-ui/core/CircularProgress';
import Grid from '@material-ui/core/Grid';

const style = theme => ({
    loading: {
        flexGrow: 2,
        flexDirection: "column",
        justifyContent: 'center',
        alignItems: 'center'
    },
})

function ChannelConfirmationLoadingPage() {

    return (<React.Fragment >
        <Grid container className={style.loading} spacing={18}>
            <Grid item xs>
                <Typography variant="subtitle1" >
                    Please wait while channel is being created...
                </Typography>
            </Grid>
            <Grid item xs={6} />
            <Grid item xs={6}>
                <CircularProgress />
            </Grid>

        </Grid>
    </React.Fragment >)


}

export default ChannelConfirmationLoadingPage;